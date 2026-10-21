use super::*;

pub fn configure_webhook_routes() -> Router<Arc<SaasService>> {
    Router::new()
        .route("/api/billing/webhook", post(handle_stripe_webhook))
}
/// `POST /api/billing/webhook`
///
/// Receives Stripe events, verifies HMAC signature and processes.
pub(super) async fn handle_stripe_webhook(
    State(service): State<Arc<SaasService>>,
    headers: HeaderMap,
    body: String,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let signature = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "missing stripe-signature header".to_string()))?;

    // Verify HMAC signature
    let event = service
        .stripe
        .verify_webhook_signature(&body, signature)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid signature: {e}")))?;

    // Parse event type
    let event_type = service
        .stripe
        .parse_webhook_event(&event)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid event: {e}")))?;

    use botbilling::stripe_integration::WebhookEventType;

    match event_type {
        WebhookEventType::CheckoutCompleted(session) => {
            // Setup-mode checkouts collect a saved card; subscription checkouts
            // pay an invoice. Dispatch accordingly.
            if session.mode == "setup" {
                crate::payment_cards::sync_checkout_setup(&service, &session)
                    .await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
            } else {
                process_checkout_completed(&service, &session).await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
            }
        }
        WebhookEventType::PaymentMethodAttached(pm) => {
            crate::payment_cards::sync_payment_method(&service, &pm, false)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
        }
        WebhookEventType::PaymentMethodUpdated(pm) => {
            crate::payment_cards::sync_payment_method(&service, &pm, false)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
        }
        WebhookEventType::PaymentMethodDetached(pm) => {
            crate::payment_cards::sync_payment_method(&service, &pm, true)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
        }
        WebhookEventType::InvoicePaid(invoice) => {
            process_invoice_paid(&service, &invoice).await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
        }
        WebhookEventType::InvoicePaymentFailed(invoice) => {
            process_invoice_payment_failed(&service, &invoice).await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
        }
        WebhookEventType::SubscriptionCreated(sub) => {
            process_subscription_lifecycle(
                &service,
                &sub.id,
                &sub.customer,
                &sub.metadata,
                "active",
            )
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
        }
        WebhookEventType::SubscriptionUpdated(sub) => {
            let status = match sub.status {
                botbilling::stripe_integration::StripeSubscriptionStatus::Active
                | botbilling::stripe_integration::StripeSubscriptionStatus::Trialing => "active",
                botbilling::stripe_integration::StripeSubscriptionStatus::PastDue => "past_due",
                botbilling::stripe_integration::StripeSubscriptionStatus::Canceled
                | botbilling::stripe_integration::StripeSubscriptionStatus::IncompleteExpired
                | botbilling::stripe_integration::StripeSubscriptionStatus::Unpaid => "cancelled",
                botbilling::stripe_integration::StripeSubscriptionStatus::Paused => "paused",
                botbilling::stripe_integration::StripeSubscriptionStatus::Incomplete => {
                    "incomplete"
                }
            };
            process_subscription_lifecycle(
                &service,
                &sub.id,
                &sub.customer,
                &sub.metadata,
                status,
            )
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
        }
        WebhookEventType::SubscriptionCanceled(sub) => {
            process_subscription_lifecycle(
                &service,
                &sub.id,
                &sub.customer,
                &sub.metadata,
                "cancelled",
            )
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
        }
        WebhookEventType::Unknown(event_type) => {
            tracing::info!("Ignoring unhandled Stripe event: {event_type}");
        }
    }

    Ok(Json(serde_json::json!({ "received": true })))
}
/// Processes `checkout.session.completed`.
///
/// 1. Updates invoice to "paid"
/// 2. Marks CRM deal as "won"
/// 3. Creates general ledger entry in GL
/// 4. Creates recurring subscription in billing_recurring
pub(super) async fn process_checkout_completed(
    service: &SaasService,
    session: &botbilling::stripe_integration::StripeCheckoutSession,
) -> Result<(), String> {
    // Extract invoice_id and plan from session metadata
    let metadata = session
        .metadata
        .as_ref()
        .ok_or_else(|| "missing metadata on checkout session".to_string())?;

    let invoice_id_str = metadata
        .get("invoice_id")
        .ok_or_else(|| "missing invoice_id in metadata".to_string())?;
    let invoice_id = Uuid::parse_str(invoice_id_str)
        .map_err(|e| format!("invalid invoice_id: {e}"))?;

    let now = chrono::Utc::now();

    // 1. Update invoice to "paid"
    {
        let mut conn = service
            .pool()
            .get()
            .map_err(|e| format!("DB pool: {e}"))?;

        diesel::update(
            botbilling::schema::billing_invoices::table
                .filter(botbilling::schema::billing_invoices::id.eq(invoice_id)),
        )
        .set((
            botbilling::schema::billing_invoices::status.eq("paid"),
            botbilling::schema::billing_invoices::paid_at.eq(Some(now)),
            botbilling::schema::billing_invoices::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .map_err(|e| format!("update invoice: {e}"))?;
    }

    // Find invoice to get customer data
    let invoice = {
        let mut conn = service
            .pool()
            .get()
            .map_err(|e| format!("DB pool: {e}"))?;
        botbilling::schema::billing_invoices::table
            .filter(botbilling::schema::billing_invoices::id.eq(invoice_id))
            .first::<botbilling::api_models::BillingInvoice>(&mut conn)
            .map_err(|e| format!("invoice not found: {e}"))?
    };

    let total = botbilling::api_models::bd_to_f64(&invoice.total);
    let plan_label = metadata.get("plan").cloned().unwrap_or_else(|| "unknown".to_string());

    // 2. CRM: mark deal as won
    let deal_name = format!("Subscription {} - {}", plan_label, invoice.customer_name.as_str());
    let _ = integration::win_crm_deal(service.pool(), invoice.branch_id, &deal_name)
        .map_err(|e| tracing::warn!("CRM win deal failed: {e}"));

    // 3. ERP (GL): posts accounting entry
    let _ = integration::create_gl_entry_for_invoice(
        service.pool(),
        invoice_id,
        total,
        invoice.customer_name.as_str(),
    )
    .map_err(|e| tracing::warn!("GL entry failed: {e}"));

    // 4. Subscription: create or update billing_recurring
    let email_ref = invoice.customer_email.clone().unwrap_or_default();
    let _ = integration::create_billing_subscription(integration::NewSubscription {
        pool: service.pool(),
        branch_id: invoice.branch_id,
        customer_name: invoice.customer_name.as_str(),
        customer_email: &email_ref,
        plan: &plan_label,
        total,
        currency: invoice.currency.as_str(),
        invoice_id,
        period: "monthly",
    })
    .map_err(|e| tracing::warn!("Subscription creation failed: {e}"));

    // --- Notifications ---
    let mut vars = notifier::EmailVars::new(
        invoice.customer_name.as_str(),
        invoice.customer_email.as_deref().unwrap_or(""),
        &plan_label,
        total,
        invoice.currency.as_str(),
    );
    vars.invoice_id = invoice_id.to_string();
    notifier::notify_payment_success(&vars);
    notifier::notify_subscription_activated(&vars);

    tracing::info!(
        "checkout.session.completed: invoice={invoice_id}, plan={plan_label}, total={total}"
    );

    Ok(())
}
