use super::*;

/// Processes `invoice.paid` — recurring charge paid successfully.
///
/// 1. Generates new invoice for next period
/// 2. Updates billing_recurring
/// 3. Creates general ledger entry in GL
pub(super) async fn process_invoice_paid(
    service: &SaasService,
    stripe_invoice: &botbilling::stripe_integration::StripeInvoice,
) -> Result<(), String> {
    let now = chrono::Utc::now();
    let customer_email = stripe_invoice
        .customer_email
        .as_deref()
        .ok_or_else(|| "invoice has no customer_email".to_string())?;

    let mut conn = service
        .pool()
        .get()
        .map_err(|e| format!("DB pool: {e}"))?;

    // Find subscription by customer email (unique per email in single-tenant model)
    let recurring = botbilling::schema::billing_recurring::table
        .filter(botbilling::schema::billing_recurring::customer_email.eq(Some(customer_email.to_string())))
        .filter(botbilling::schema::billing_recurring::status.eq("active"))
        .first::<botbilling::api_models::BillingRecurring>(&mut conn)
        .map_err(|_| format!("billing_recurring not found for email {customer_email}"))?;

    let total = recurring.amount.as_ref().map(botbilling::api_models::bd_to_f64).unwrap_or(0.0);

    // Generate new invoice for the period
    let new_invoice_id = Uuid::new_v4();
    let invoice_number =
        botbilling::api_models::generate_invoice_number(&mut conn, recurring.branch_id);

    let (invoice_org_id, invoice_bot_id) = botbilling::scope_for_branch(&mut conn, recurring.branch_id);

    let new_invoice = botbilling::api_models::BillingInvoice {
        id: new_invoice_id,
        branch_id: recurring.branch_id,
        invoice_number,
        customer_id: None,
        customer_name: Some(recurring.customer_name.clone()),
        customer_email: recurring.customer_email.clone(),
        customer_address: None,
        status: Some("paid".to_string()),
        issue_date: now.date_naive(),
        due_date: Some((now + chrono::Duration::days(30)).date_naive()),
        subtotal: recurring.amount.clone().unwrap_or_else(|| botbilling::api_models::bd(0.0)),
        tax_rate: botbilling::api_models::bd(0.0),
        tax_amount: botbilling::api_models::bd(0.0),
        discount_percent: botbilling::api_models::bd(0.0),
        discount_amount: botbilling::api_models::bd(0.0),
        total: recurring.amount.clone(),
        amount_paid: recurring.amount.clone().unwrap_or_else(|| botbilling::api_models::bd(0.0)),
        amount_due: botbilling::api_models::bd(0.0),
        currency: recurring.currency.clone(),
        notes: Some(format!("Recurring charge - {}", recurring.description.as_deref().unwrap_or("subscription"))),
        terms: None,
        footer: None,
        paid_at: Some(now),
        sent_at: None,
        voided_at: None,
        created_at: now,
        updated_at: now,
    org_id: invoice_org_id, bot_id: invoice_bot_id,
    };

    diesel::insert_into(botbilling::schema::billing_invoices::table)
        .values(&new_invoice)
        .execute(&mut conn)
        .map_err(|e| format!("insert invoice: {e}"))?;

    let line_item = botbilling::api_models::BillingInvoiceItem {
        id: Uuid::new_v4(),
        invoice_id: new_invoice_id,
        product_id: None,
        description: recurring.description.clone().unwrap_or_else(|| "Subscription".to_string()),
        quantity: botbilling::api_models::bd(1.0),
        unit_price: recurring.amount.clone().unwrap_or_else(|| botbilling::api_models::bd(0.0)),
        discount_percent: botbilling::api_models::bd(0.0),
        tax_rate: botbilling::api_models::bd(0.0),
        amount: recurring.amount.clone().unwrap_or_else(|| botbilling::api_models::bd(0.0)),
        sort_order: 0,
        created_at: now,
    };

    diesel::insert_into(botbilling::schema::billing_invoice_items::table)
        .values(&line_item)
        .execute(&mut conn)
        .map_err(|e| format!("insert invoice item: {e}"))?;

    // Update billing_recurring
    let _next_date = if recurring.frequency == "yearly" {
        now.date_naive() + chrono::Duration::days(365)
    } else {
        now.date_naive() + chrono::Duration::days(30)
    };

    diesel::update(
        botbilling::schema::billing_recurring::table
            .filter(botbilling::schema::billing_recurring::id.eq(recurring.id)),
    )
    .set((
        botbilling::schema::billing_recurring::last_invoice_date.eq(Some(now.date_naive())),
        botbilling::schema::billing_recurring::last_invoice_id.eq(Some(new_invoice_id)),
        botbilling::schema::billing_recurring::next_invoice_date.eq(now.date_naive()),
        botbilling::schema::billing_recurring::invoices_generated
            .eq(recurring.invoices_generated + 1),
        botbilling::schema::billing_recurring::updated_at.eq(now),
    ))
    .execute(&mut conn)
    .map_err(|e| format!("update recurring: {e}"))?;

    // GL
    let _ = integration::create_gl_entry_for_invoice(
        service.pool(),
        new_invoice_id,
        total,
        &recurring.customer_name,
    )
    .map_err(|e| tracing::warn!("GL entry for recurring failed: {e}"));

    // --- Notification: recurring charge ---
    let mut vars = notifier::EmailVars::new(
        &recurring.customer_name,
        recurring.customer_email.as_deref().unwrap_or(""),
        &recurring.description.clone().unwrap_or_else(|| "subscription".to_string()),
        total,
        recurring.currency.as_deref().unwrap_or(""),
    );
    vars.invoice_id = new_invoice_id.to_string();
    notifier::notify_recurring_charge(&vars);

    tracing::info!(
        "invoice.paid: stripe={}, new_invoice={}, amount={}",
        stripe_invoice.id,
        new_invoice_id,
        total
    );

    Ok(())
}
/// Processes `invoice.payment_failed`.
///
/// Marks the invoice as "overdue" and the subscription as "past_due".
pub(super) async fn process_invoice_payment_failed(
    service: &SaasService,
    stripe_invoice: &botbilling::stripe_integration::StripeInvoice,
) -> Result<(), String> {
    let now = chrono::Utc::now();
    let customer_email = stripe_invoice
        .customer_email
        .as_deref()
        .ok_or_else(|| "invoice has no customer_email".to_string())?;

    let mut conn = service
        .pool()
        .get()
        .map_err(|e| format!("DB pool: {e}"))?;

    let recurring = botbilling::schema::billing_recurring::table
        .filter(botbilling::schema::billing_recurring::customer_email.eq(Some(customer_email.to_string())))
        .filter(botbilling::schema::billing_recurring::status.eq("active"))
        .first::<botbilling::api_models::BillingRecurring>(&mut conn)
        .map_err(|_| format!("billing_recurring not found for email {customer_email}"))?;

    // Mark last invoice as overdue
    if let Some(last_inv_id) = recurring.last_invoice_id {
        let _ = diesel::update(
            botbilling::schema::billing_invoices::table
                .filter(botbilling::schema::billing_invoices::id.eq(last_inv_id)),
        )
        .set((
            botbilling::schema::billing_invoices::status.eq("overdue"),
            botbilling::schema::billing_invoices::updated_at.eq(now),
        ))
        .execute(&mut conn);
    }

    // Mark subscription as past_due
    let _ = diesel::update(
        botbilling::schema::billing_recurring::table
            .filter(botbilling::schema::billing_recurring::id.eq(recurring.id)),
    )
    .set((
        botbilling::schema::billing_recurring::status.eq("past_due"),
        botbilling::schema::billing_recurring::updated_at.eq(now),
    ))
    .execute(&mut conn);

    let amount_f64 = recurring.amount.as_ref().map(botbilling::api_models::bd_to_f64).unwrap_or(0.0);
    let mut vars = notifier::EmailVars::new(
        &recurring.customer_name,
        recurring.customer_email.as_deref().unwrap_or(""),
        &recurring.description.clone().unwrap_or_else(|| "subscription".to_string()),
        amount_f64,
        recurring.currency.as_deref().unwrap_or(""),
    );
    vars.invoice_id = recurring.last_invoice_id.unwrap_or(Uuid::nil()).to_string();
    notifier::notify_payment_failed(&vars);

    tracing::warn!(
        "invoice.payment_failed: stripe={}, email={customer_email}",
        stripe_invoice.id,
    );

    Ok(())
}
/// Processes subscription lifecycle events.
///
/// Creates or updates `billing_recurring` with Stripe subscription data.
pub(super) async fn process_subscription_lifecycle(
    service: &SaasService,
    stripe_subscription_id: &str,
    stripe_customer_id: &str,
    metadata: &std::collections::HashMap<String, String>,
    status: &str,
) -> Result<(), String> {
    let now = chrono::Utc::now();
    let mut conn = service
        .pool()
        .get()
        .map_err(|e| format!("DB pool: {e}"))?;

    // Try to find billing_recurring by stored stripe_subscription_id
    // Since we don't have a column for this, use customer email from metadata
    let account_email = metadata
        .get("account_email")
        .cloned()
        .unwrap_or_default();

    if status == "cancelled" || status == "past_due" || status == "paused" {
        let _ = diesel::update(
            botbilling::schema::billing_recurring::table
                .filter(botbilling::schema::billing_recurring::customer_email.eq(Some(account_email.clone()))),
        )
        .set((
            botbilling::schema::billing_recurring::status.eq(status),
            botbilling::schema::billing_recurring::updated_at.eq(now),
        ))
        .execute(&mut conn);

        if status == "cancelled" && !account_email.is_empty() {
            let vars = notifier::EmailVars::new(
                &account_email, &account_email, "unknown", 0.0, "USD",
            );
            notifier::notify_subscription_cancelled(&vars);
        }
    }

    tracing::info!(
        "subscription.lifecycle: stripe_sub={stripe_subscription_id}, customer={stripe_customer_id}, status={status}"
    );

    Ok(())
}
