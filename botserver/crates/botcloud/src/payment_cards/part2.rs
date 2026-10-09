use super::*;

/// `POST /api/cloud/payment-cards/:pm_id/default`
///
/// Sets the default payment method on the Stripe customer and mirrors the flag
/// locally. Only the branch owner (or super-admin) may change the default.
pub async fn set_default_payment_card(
    State(service): State<Arc<SaasService>>,
    Path(pm_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut conn = service
        .pool()
        .get()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB: {e}")))?;
    let branch_id = get_branch_id_from_jwt(&headers, &mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let admin = is_super_admin(&headers, &mut conn).unwrap_or(false);
    let email = jwt_email(&headers);

    use crate::schema_ext::billing_payment_methods::dsl::{
        billing_payment_methods, branch_id as pm_branch, stripe_customer_id as pm_customer,
        stripe_pm_id, is_default, updated_at,
    };
    let row: Option<(Uuid, String)> = billing_payment_methods
        .filter(stripe_pm_id.eq(&pm_id))
        .select((pm_branch, pm_customer))
        .first(&mut conn)
        .optional()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query: {e}")))?;

    let Some((card_branch, customer_id)) = row else {
        return Err((StatusCode::NOT_FOUND, "Card not found".to_string()));
    };
    if branch_id != Some(card_branch) && !admin {
        return Err((StatusCode::FORBIDDEN, "Card does not belong to this account".to_string()));
    }

    service
        .stripe
        .set_default_payment_method(&customer_id, &pm_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Stripe: {e}")))?;

    // Clear the previous default within the branch, then mark the new one.
    let now = chrono::Utc::now();
    diesel::update(billing_payment_methods.filter(pm_branch.eq(card_branch)))
        .set(is_default.eq(false))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update defaults: {e}")))?;
    diesel::update(billing_payment_methods.filter(stripe_pm_id.eq(&pm_id)))
        .set((is_default.eq(true), updated_at.eq(now)))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update card: {e}")))?;

    record_cloud_audit(
        &service,
        card_branch,
        email.as_deref(),
        "card.set_default",
        "payment_method",
        Some(&pm_id),
        None,
    );

    Ok(Json(serde_json::json!({ "ok": true, "default": pm_id })))
}
/// `DELETE /api/cloud/payment-cards/:pm_id`
///
/// Detaches the card from the Stripe customer and removes the local record.
/// The last card of a branch is never auto-unset; Stripe requires at least the
/// default handling on its side, which the portal covers.
pub async fn delete_payment_card(
    State(service): State<Arc<SaasService>>,
    Path(pm_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut conn = service
        .pool()
        .get()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB: {e}")))?;
    let branch_id = get_branch_id_from_jwt(&headers, &mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let admin = is_super_admin(&headers, &mut conn).unwrap_or(false);
    let email = jwt_email(&headers);

    use crate::schema_ext::billing_payment_methods::dsl::{
        billing_payment_methods, branch_id as pm_branch, stripe_customer_id as pm_customer,
        stripe_pm_id, is_default, created_at,
    };
    let row: Option<(Uuid, String, bool)> = billing_payment_methods
        .filter(stripe_pm_id.eq(&pm_id))
        .select((pm_branch, pm_customer, is_default))
        .first(&mut conn)
        .optional()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query: {e}")))?;

    let Some((card_branch, customer_id, was_default)) = row else {
        return Err((StatusCode::NOT_FOUND, "Card not found".to_string()));
    };
    if branch_id != Some(card_branch) && !admin {
        return Err((StatusCode::FORBIDDEN, "Card does not belong to this account".to_string()));
    }

    service
        .stripe
        .detach_payment_method(&pm_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Stripe detach: {e}")))?;

    diesel::delete(billing_payment_methods.filter(stripe_pm_id.eq(&pm_id)))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Delete: {e}")))?;

    // If the default was deleted, promote the oldest remaining card.
    if was_default {
        let next: Option<String> = billing_payment_methods
            .filter(pm_branch.eq(card_branch))
            .select(stripe_pm_id)
            .order(created_at.asc())
            .first(&mut conn)
            .optional()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query next: {e}")))?;
        if let Some(next_pm) = next {
            if let Err(e) = service.stripe.set_default_payment_method(&customer_id, &next_pm).await {
                tracing::warn!("promote default after delete failed: {e}");
            } else {
                diesel::update(billing_payment_methods.filter(stripe_pm_id.eq(&next_pm)))
                    .set(is_default.eq(true))
                    .execute(&mut conn)
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Promote: {e}")))?;
            }
        }
    }

    record_cloud_audit(
        &service,
        card_branch,
        email.as_deref(),
        "card.delete",
        "payment_method",
        Some(&pm_id),
        None,
    );

    Ok(Json(serde_json::json!({ "ok": true, "deleted": pm_id })))
}

// ─────────────────────────────────────────────────────────────────────────────
// Webhook-driven sync (keeps our records consistent with Stripe's source of truth)
// ─────────────────────────────────────────────────────────────────────────────
/// Upserts (or removes) a `billing_payment_methods` row from a Stripe
/// PaymentMethod payload. Resolves the owning branch via `billing_customers`.
pub async fn sync_payment_method(
    service: &SaasService,
    pm: &botbilling::stripe_integration::StripePaymentMethod,
    detach: bool,
) -> Result<(), String> {
    let customer_id = pm
        .customer
        .as_deref()
        .ok_or_else(|| "payment method has no customer".to_string())?;

    let branch_id = {
        let mut conn = service.pool().get().map_err(|e| format!("DB pool: {e}"))?;
        use crate::schema_ext::billing_customers::dsl::{
            billing_customers, branch_id as bc_branch, stripe_customer_id as bc_stripe,
        };
        billing_customers
            .filter(bc_stripe.eq(customer_id))
            .select(bc_branch)
            .first::<Uuid>(&mut conn)
            .optional()
            .map_err(|e| format!("Query billing_customers: {e}"))?
    };
    let Some(branch_id) = branch_id else {
        tracing::warn!(
            "payment_method webhook: no local customer mapping for {customer_id}, skipping"
        );
        return Ok(());
    };

    let mut conn = service.pool().get().map_err(|e| format!("DB pool: {e}"))?;
    use crate::schema_ext::billing_payment_methods::dsl::{
        billing_payment_methods, stripe_pm_id, brand, last4, exp_month, exp_year, is_default,
        stripe_customer_id as pm_customer, branch_id as pm_branch, id, created_at, updated_at,
    };

    if detach {
        diesel::delete(billing_payment_methods.filter(stripe_pm_id.eq(&pm.id)))
            .execute(&mut conn)
            .map_err(|e| format!("Delete payment method: {e}"))?;
        record_cloud_audit(service, branch_id, None, "card.delete.webhook", "payment_method", Some(&pm.id), None);
        return Ok(());
    }

    let Some(card) = pm.card.as_ref() else {
        tracing::debug!("payment_method {} is not a card, skipping sync", pm.id);
        return Ok(());
    };

    let now = chrono::Utc::now();
    let brand_name = card.brand.clone();
    let last = card.last4.clone();
    let exp_m = card.exp_month as i32;
    let exp_y = card.exp_year as i32;

    let inserted = diesel::insert_into(billing_payment_methods)
        .values((
            id.eq(Uuid::new_v4()),
            pm_branch.eq(branch_id),
            pm_customer.eq(customer_id),
            stripe_pm_id.eq(&pm.id),
            brand.eq(&brand_name),
            last4.eq(&last),
            exp_month.eq(exp_m),
            exp_year.eq(exp_y),
            is_default.eq(false),
            created_at.eq(now),
            updated_at.eq(now),
        ))
        .on_conflict(stripe_pm_id)
        .do_update()
        .set((
            brand.eq(&brand_name),
            last4.eq(&last),
            exp_month.eq(exp_m),
            exp_year.eq(exp_y),
            updated_at.eq(now),
        ))
        .execute(&mut conn)
        .map_err(|e| format!("Upsert payment method: {e}"))?;

    // First card of the branch becomes the default automatically.
    if inserted == 1 {
        let count: i64 = billing_payment_methods
            .filter(pm_branch.eq(branch_id))
            .count()
            .get_result(&mut conn)
            .map_err(|e| format!("Count cards: {e}"))?;
        if count == 1 {
            diesel::update(billing_payment_methods.filter(stripe_pm_id.eq(&pm.id)))
                .set(is_default.eq(true))
                .execute(&mut conn)
                .map_err(|e| format!("Set first default: {e}"))?;
        }
    }

    record_cloud_audit(service, branch_id, None, "card.add.webhook", "payment_method", Some(&pm.id), None);
    Ok(())
}
/// Handles `checkout.session.completed` for `mode=setup`: resolves the attached
/// payment method from the SetupIntent and syncs it locally.
pub async fn sync_checkout_setup(
    service: &SaasService,
    session: &botbilling::stripe_integration::StripeCheckoutSession,
) -> Result<(), String> {
    if session.mode != "setup" {
        return Ok(());
    }
    let setup_intent_id = session
        .setup_intent
        .as_deref()
        .ok_or_else(|| "setup session without setup_intent".to_string())?;

    let intent = service
        .stripe
        .retrieve_setup_intent(setup_intent_id)
        .await
        .map_err(|e| format!("Retrieve SetupIntent: {e}"))?;

    let pm_id = intent
        .payment_method
        .ok_or_else(|| "SetupIntent has no payment method".to_string())?;

    // The PaymentMethod is already attached to the customer by setup mode.
    // Fetch its details for the local mirror.
    let customer_id = intent
        .customer
        .as_deref()
        .ok_or_else(|| "SetupIntent has no customer".to_string())?;

    let methods = service
        .stripe
        .get_payment_methods(customer_id)
        .await
        .map_err(|e| format!("List payment methods: {e}"))?;
    let pm = methods
        .into_iter()
        .find(|m| m.id == pm_id)
        .ok_or_else(|| format!("payment method {pm_id} not found for customer"))?;

    sync_payment_method(service, &pm, false).await?;
    Ok(())
}
