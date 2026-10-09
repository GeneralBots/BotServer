use super::*;

/// Extracts the `email` claim from the Bearer JWT, if present.
pub fn jwt_email(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .and_then(|token| {
            let parts: Vec<&str> = token.split('.').collect();
            if parts.len() == 3 {
                base64_url_decode(parts[1])
                    .ok()
                    .and_then(|decoded| serde_json::from_slice::<serde_json::Value>(&decoded).ok())
                    .and_then(|payload| payload.get("email").and_then(|v| v.as_str()).map(str::to_string))
            } else {
                None
            }
        })
}
/// Records one durable billing audit event (`cloud_audit_log`).
pub fn record_cloud_audit(
    service: &SaasService,
    branch_id: Uuid,
    actor_email: Option<&str>,
    action: &str,
    entity: &str,
    entity_id: Option<&str>,
    details: Option<&str>,
) {
    let Some(mut conn) = service.pool().get().ok() else {
        tracing::warn!("billing audit: DB unavailable, event dropped ({action})");
        return;
    };
    let actor = actor_email.map(str::to_string);
    let entity_id_owned = entity_id.map(str::to_string);
    let details_owned = details.map(str::to_string);
    let result = diesel::sql_query(
        "INSERT INTO cloud_audit_log \
         (branch_id, actor_email, action, entity, entity_id, details) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch_id)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(actor)
    .bind::<diesel::sql_types::Text, _>(action)
    .bind::<diesel::sql_types::Text, _>(entity)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(entity_id_owned)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(details_owned)
    .execute(&mut conn);
    if let Err(e) = result {
        tracing::error!("billing audit: failed to persist event ({action}): {e}");
    }
}
/// Resolves the Stripe Customer id for a branch, creating and persisting the
/// customer mapping when one does not exist yet. Persisted in `billing_customers`
/// so webhook events carrying only a `customer` id can be re-attributed.
pub async fn resolve_stripe_customer(
    service: &SaasService,
    email: &str,
    branch_id: Uuid,
) -> Result<String, String> {
    // 1. Cached mapping in our database.
    {
        let mut conn = service
            .pool()
            .get()
            .map_err(|e| format!("DB pool: {e}"))?;
        use crate::schema_ext::billing_customers::dsl::{
            billing_customers, branch_id as bc_branch, stripe_customer_id as bc_stripe,
        };
        let existing: Option<String> = billing_customers
            .filter(bc_branch.eq(branch_id))
            .select(bc_stripe)
            .first(&mut conn)
            .optional()
            .map_err(|e| format!("Query billing_customers: {e}"))?;
        if let Some(customer_id) = existing {
            return Ok(customer_id);
        }
    }

    // 2. Ask Stripe whether a customer for this email already exists.
    if let Some(customer) = service
        .stripe
        .find_customer_by_email(email)
        .await
        .map_err(|e| format!("Stripe lookup: {e}"))?
    {
        persist_customer_mapping(service, branch_id, &customer.id, email)?;
        return Ok(customer.id);
    }

    // 3. No customer yet — create one and remember it.
    let customer = service
        .stripe
        .create_customer(botbilling::stripe_integration::CreateCustomerParams {
            email: email.to_string(),
            name: None,
            organization_id: branch_id,
            metadata: std::collections::HashMap::new(),
        })
        .await
        .map_err(|e| format!("Stripe customer: {e}"))?;
    persist_customer_mapping(service, branch_id, &customer.id, email)?;
    Ok(customer.id)
}
pub(crate) fn persist_customer_mapping(
    service: &SaasService,
    branch_id: Uuid,
    stripe_customer_id: &str,
    email: &str,
) -> Result<(), String> {
    let mut conn = service.pool().get().map_err(|e| format!("DB pool: {e}"))?;
    use crate::schema_ext::billing_customers::dsl::{
        billing_customers, id as bc_id, branch_id as bc_branch, email as bc_email,
        stripe_customer_id as bc_stripe,
    };
    diesel::insert_into(billing_customers)
        .values((
            bc_id.eq(Uuid::new_v4()),
            bc_branch.eq(branch_id),
            bc_stripe.eq(stripe_customer_id),
            bc_email.eq(email),
        ))
        .on_conflict(bc_branch)
        .do_update()
        .set((
            bc_stripe.eq(stripe_customer_id),
            bc_email.eq(email),
        ))
        .execute(&mut conn)
        .map_err(|e| format!("Upsert billing_customers: {e}"))?;
    Ok(())
}
/// `GET /api/cloud/payment-cards`
///
/// Lists saved cards for the authenticated user's branch, read from our local
/// table (kept in sync by Stripe webhooks). Super-admins without a branch see
/// every saved card.
pub async fn list_payment_cards(
    State(service): State<Arc<SaasService>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut conn = service
        .pool()
        .get()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB: {e}")))?;

    let user_branch_id = get_branch_id_from_jwt(&headers, &mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let admin = is_super_admin(&headers, &mut conn).unwrap_or(false);

    use crate::schema_ext::billing_payment_methods::dsl::{
        billing_payment_methods, id, stripe_pm_id, brand, last4, exp_month, exp_year, is_default,
        stripe_customer_id, branch_id, created_at,
    };
    let mut query = billing_payment_methods
        .select((
            id,
            stripe_pm_id,
            brand,
            last4,
            exp_month,
            exp_year,
            is_default,
            stripe_customer_id,
        ))
        .into_boxed();

    if let Some(branch) = user_branch_id {
        query = query.filter(branch_id.eq(branch));
    } else if !admin {
        return Err((StatusCode::FORBIDDEN, "No billing scope for this account".to_string()));
    }

    let rows = query
        .order(is_default.desc())
        .order(created_at.asc())
        .load::<(Uuid, String, String, String, i32, i32, bool, String)>(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query: {e}")))?;

    let cards: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|(card_id, pm_id, brand_name, last, exp_m, exp_y, is_def, cust)| {
            serde_json::json!({
                "id": card_id,
                "stripe_pm_id": pm_id,
                "brand": brand_name,
                "last4": last,
                "exp_month": exp_m,
                "exp_year": exp_y,
                "is_default": is_def,
                "customer_id": cust,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "cards": cards })))
}
/// `POST /api/cloud/payment-cards/setup`
///
/// Creates a hosted Stripe Checkout session in `setup` mode. The browser is
/// redirected to `url`; Stripe collects the card on its own domain and fires
/// `checkout.session.completed` (mode=setup) when done.
pub async fn create_payment_card_setup(
    State(service): State<Arc<SaasService>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut conn = service
        .pool()
        .get()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB: {e}")))?;
    let branch_id = get_branch_id_from_jwt(&headers, &mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or_else(|| (StatusCode::FORBIDDEN, "No billing scope for this account".to_string()))?;
    let email = jwt_email(&headers)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Missing email claim".to_string()))?;

    let customer_id = resolve_stripe_customer(&service, &email, branch_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let base = service.config.base_url.trim_end_matches('/').to_string();
    let session = service
        .stripe
        .create_setup_checkout_session(
            botbilling::stripe_integration::CreateSetupCheckoutSessionParams {
                customer_id,
                success_url: format!("{base}/payment-cards?added=1"),
                cancel_url: format!("{base}/payment-cards?cancelled=1"),
                metadata: std::collections::HashMap::from([
                    ("branch_id".to_string(), branch_id.to_string()),
                    ("email".to_string(), email.clone()),
                ]),
            },
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Stripe session: {e}")))?;

    record_cloud_audit(
        &service,
        branch_id,
        Some(email.as_str()),
        "card.add.initiated",
        "payment_method",
        None,
        Some(&format!("checkout_session={}", session.id)),
    );

    Ok(Json(serde_json::json!({ "url": session.url, "session_id": session.id })))
}
/// `POST /api/cloud/payment-cards/setup-intent`
///
/// Creates a raw SetupIntent and returns the `client_secret` for clients that
/// embed Stripe Elements directly (headless / native integrations). The card is
/// attached and synced via the `payment_method.attached` webhook.
pub async fn create_payment_card_setup_intent(
    State(service): State<Arc<SaasService>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut conn = service
        .pool()
        .get()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB: {e}")))?;
    let branch_id = get_branch_id_from_jwt(&headers, &mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
        .ok_or_else(|| (StatusCode::FORBIDDEN, "No billing scope for this account".to_string()))?;
    let email = jwt_email(&headers)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Missing email claim".to_string()))?;

    let customer_id = resolve_stripe_customer(&service, &email, branch_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let intent = service
        .stripe
        .create_setup_intent(botbilling::stripe_integration::CreateSetupIntentParams {
            customer_id: customer_id.clone(),
            usage: "off_session".to_string(),
        })
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Stripe SetupIntent: {e}")))?;

    Ok(Json(serde_json::json!({
        "client_secret": intent.client_secret,
        "setup_intent_id": intent.id,
        "customer_id": customer_id,
    })))
}
