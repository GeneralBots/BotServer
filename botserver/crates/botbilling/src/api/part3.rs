use super::*;

pub async fn accept_quote(
    State(state): State<Arc<BillingApiState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<BillingQuote>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let now = Utc::now();

    diesel::update(billing_quotes::table.filter(billing_quotes::id.eq(id)))
        .set((
            billing_quotes::status.eq("accepted"),
            billing_quotes::accepted_at.eq(Some(now)),
            billing_quotes::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    let quote: BillingQuote = billing_quotes::table
        .filter(billing_quotes::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Quote not found".to_string()))?;

    Ok(Json(quote))
}
pub async fn reject_quote(
    State(state): State<Arc<BillingApiState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<BillingQuote>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let now = Utc::now();

    diesel::update(billing_quotes::table.filter(billing_quotes::id.eq(id)))
        .set((
            billing_quotes::status.eq("rejected"),
            billing_quotes::rejected_at.eq(Some(now)),
            billing_quotes::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    let quote: BillingQuote = billing_quotes::table
        .filter(billing_quotes::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Quote not found".to_string()))?;

    Ok(Json(quote))
}
pub async fn delete_quote(
    State(state): State<Arc<BillingApiState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    diesel::delete(billing_quotes::table.filter(billing_quotes::id.eq(id)))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Delete error: {e}")))?;

    Ok(StatusCode::NO_CONTENT)
}
pub async fn get_billing_stats(
    State(state): State<Arc<BillingApiState>>,
) -> Result<Json<BillingStats>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = get_bot_context(&state.pool, &state.get_default_bot);
    let today = Utc::now().date_naive();

    let invoices: Vec<BillingInvoice> = billing_invoices::table
        .filter(billing_invoices::branch_id.eq(branch_id))
        .load(&mut conn)
        .unwrap_or_default();

    let mut total_revenue = 0.0;
    let mut pending_amount = 0.0;
    let mut overdue_amount = 0.0;
    let mut overdue_count = 0i64;

    for inv in &invoices {
        if inv.status.as_deref() == Some("paid") {
            total_revenue += inv.total.as_ref().map_or(0.0, bd_to_f64);
        }
        if inv.status.as_deref() != Some("paid") && inv.status.as_deref() != Some("voided") {
            pending_amount += bd_to_f64(&inv.amount_due);
            if inv.due_date.unwrap_or(NaiveDate::MAX) < today {
                overdue_amount += bd_to_f64(&inv.amount_due);
                overdue_count += 1;
            }
        }
    }

    let payments: Vec<BillingPayment> = billing_payments::table
        .filter(billing_payments::branch_id.eq(branch_id))
        .filter(billing_payments::status.eq("completed"))
        .load(&mut conn)
        .unwrap_or_default();

    let paid_this_month: f64 = payments
        .iter()
        .filter(|p| p.paid_at.map(|d| d.date_naive().month() == today.month() && d.date_naive().year() == today.year()).unwrap_or(false))
        .map(|p| bd_to_f64(&p.amount))
        .sum();

    let revenue_this_month: f64 = invoices
        .iter()
        .filter(|i| i.status.as_deref() == Some("paid") && i.paid_at.map(|d| d.date_naive().month() == today.month() && d.date_naive().year() == today.year()).unwrap_or(false))
        .map(|i| i.total.as_ref().map_or(0.0, bd_to_f64))
        .sum();

    let stats = BillingStats {
        total_revenue,
        revenue_this_month,
        pending_amount,
        overdue_amount,
        paid_this_month,
        invoice_count: invoices.len() as i64,
        payment_count: payments.len() as i64,
        overdue_count,
    };

    Ok(Json(stats))
}
pub async fn list_overdue_invoices(
    State(state): State<Arc<BillingApiState>>,
) -> Result<Json<Vec<BillingInvoice>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = get_bot_context(&state.pool, &state.get_default_bot);
    let today = Utc::now().date_naive();

    let invoices: Vec<BillingInvoice> = billing_invoices::table
        .filter(billing_invoices::branch_id.eq(branch_id))
        .filter(billing_invoices::status.ne("paid"))
        .filter(billing_invoices::status.ne("voided"))
        .filter(billing_invoices::due_date.lt(today))
        .order(billing_invoices::due_date.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(invoices))
}
pub async fn list_tax_rates(
    State(state): State<Arc<BillingApiState>>,
) -> Result<Json<Vec<BillingTaxRate>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = get_bot_context(&state.pool, &state.get_default_bot);

    let rates: Vec<BillingTaxRate> = billing_tax_rates::table
        .filter(billing_tax_rates::branch_id.eq(branch_id))
        .filter(billing_tax_rates::is_active.eq(true))
        .order(billing_tax_rates::name.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(rates))
}
pub async fn list_recurring(
    State(state): State<Arc<BillingApiState>>,
) -> Result<Json<Vec<BillingRecurring>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = get_bot_context(&state.pool, &state.get_default_bot);

    let recurring: Vec<BillingRecurring> = billing_recurring::table
        .filter(billing_recurring::branch_id.eq(branch_id))
        .filter(billing_recurring::status.eq("active"))
        .order(billing_recurring::next_invoice_date.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(recurring))
}
pub fn configure_billing_api_routes() -> Router<Arc<BillingApiState>> {
    Router::new()
        .route("/api/billing/invoices", get(list_invoices).post(create_invoice))
        .route("/api/billing/invoices/overdue", get(list_overdue_invoices))
        .route("/api/billing/invoices/:id", get(get_invoice).put(update_invoice).delete(delete_invoice))
        .route("/api/billing/invoices/:id/send", put(send_invoice))
        .route("/api/billing/invoices/:id/void", put(void_invoice))
        .route("/api/billing/payments", get(list_payments).post(record_payment))
        .route("/api/billing/payments/:id", get(get_payment))
        .route("/api/billing/quotes", get(list_quotes).post(create_quote))
        .route("/api/billing/quotes/:id", get(get_quote).delete(delete_quote))
        .route("/api/billing/quotes/:id/accept", put(accept_quote))
        .route("/api/billing/quotes/:id/reject", put(reject_quote))
        .route("/api/billing/stats", get(get_billing_stats))
        .route("/api/billing/tax-rates", get(list_tax_rates))
        .route("/api/billing/recurring", get(list_recurring))
}
