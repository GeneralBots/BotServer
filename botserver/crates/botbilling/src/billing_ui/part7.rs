use super::*;

pub(super) async fn handle_quotes(
    State(state): State<Arc<BillingApiState>>,
    headers: HeaderMap,
    Query(query): Query<StatusQuery>,
) -> impl IntoResponse {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot;

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;
        let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
            .unwrap_or_else(|| crate::get_bot_context(&pool, &get_default_bot));
        if branch_id == Uuid::nil() { return None; }

        let mut db_query = billing_quotes::table
            .filter(billing_quotes::branch_id.eq(branch_id))
            .into_boxed();

        if let Some(ref status) = query.status {
            db_query = db_query.filter(billing_quotes::status.eq(status));
        }

        db_query = db_query.order(billing_quotes::created_at.desc());

        let limit = query.limit.unwrap_or(50);
        db_query = db_query.limit(limit);

        db_query
            .select((
                billing_quotes::id,
                billing_quotes::quote_number,
                billing_quotes::customer_name,
                billing_quotes::customer_email,
                billing_quotes::status,
                billing_quotes::issue_date,
                billing_quotes::valid_until,
                billing_quotes::total,
                billing_quotes::currency,
            ))
            .load::<(Uuid, String, Option<String>, Option<String>, Option<String>, NaiveDate, Option<NaiveDate>, Option<BigDecimal>, Option<String>)>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    match result {
        Some(quotes) if !quotes.is_empty() => {
            let mut html = String::new();
            for (id, number, customer_name, customer_email, status, issue_date, valid_until, total, currency) in quotes {
                let name = customer_email.or(customer_name).unwrap_or_else(|| "Unknown".to_string());
                let total_str = format_currency(bd_to_f64(&total.unwrap_or_default()), &currency.as_deref().unwrap_or("USD"));
                let issue_str = issue_date.format("%Y-%m-%d").to_string();
                let valid_str = valid_until.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "-".to_string());

                let status_class = match status.as_deref().unwrap_or("draft") {
                    "accepted" => "status-accepted",
                    "sent" => "status-sent",
                    "rejected" => "status-rejected",
                    "expired" => "status-expired",
                    "converted" => "status-converted",
                    _ => "status-draft",
                };

                html.push_str(&format!(
                    r##"<tr class="quote-row" data-id="{id}"><td class="quote-number">{}</td><td class="quote-customer">{}</td><td class="quote-date">{}</td><td class="quote-valid">{}</td><td class="quote-total">{}</td><td class="quote-status"><span class="{}">{}</span></td><td class="quote-actions"><button class="btn-sm" hx-get="/api/billing/quotes/{id}" hx-target="#quote-detail">View</button></td></tr>"##,
                    html_escape(&number),
                    html_escape(&name),
                    issue_str,
                    valid_str,
                    total_str,
                    status_class,
                    html_escape(status.as_deref().unwrap_or(""))
                ));
            }
            Html(html)
        }
        _ => Html(r#"<tr class="empty-row"><td colspan="7" class="empty-state"><div class="empty-icon">Quote</div><p>No quotes yet</p><p class="empty-hint">Create quotes for your prospects</p></td></tr>"#.to_string()),
    }
}
pub(super) async fn handle_stats_pending(State(state): State<Arc<BillingApiState>>, headers: HeaderMap) -> impl IntoResponse {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot;

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;
        let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
            .unwrap_or_else(|| crate::get_bot_context(&pool, &get_default_bot));
        if branch_id == Uuid::nil() { return None; }

        let totals: Vec<BigDecimal> = billing_invoices::table
            .filter(billing_invoices::branch_id.eq(branch_id))
            .filter(billing_invoices::status.eq_any(vec!["sent", "draft"]))
            .select(billing_invoices::amount_due)
            .load(&mut conn)
            .ok()?;

        let sum: f64 = totals.iter().map(bd_to_f64).sum();
        Some(sum)
    })
    .await
    .ok()
    .flatten();

    Html(format_currency(result.unwrap_or(0.0), "USD"))
}
pub(super) async fn handle_revenue_month(State(state): State<Arc<BillingApiState>>, headers: HeaderMap) -> impl IntoResponse {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot;

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;
        let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
            .unwrap_or_else(|| crate::get_bot_context(&pool, &get_default_bot));
        if branch_id == Uuid::nil() { return None; }

        let now = Utc::now();
        let month_start = now.date_naive().with_day(1)?.and_hms_opt(0, 0, 0)?;

        let totals: Vec<Option<BigDecimal>> = billing_invoices::table
            .filter(billing_invoices::branch_id.eq(branch_id))
            .filter(billing_invoices::created_at.ge(month_start))
            .select(billing_invoices::total)
            .load(&mut conn)
            .ok()?;

        let sum: f64 = totals.iter().map(|t| t.as_ref().map_or(0.0, |v| bd_to_f64(v))).sum();
        Some(sum)
    })
    .await
    .ok()
    .flatten();

    Html(format_currency(result.unwrap_or(0.0), "USD"))
}
pub(super) async fn handle_paid_month(State(state): State<Arc<BillingApiState>>, headers: HeaderMap) -> impl IntoResponse {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot;

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;
        let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
            .unwrap_or_else(|| crate::get_bot_context(&pool, &get_default_bot));
        if branch_id == Uuid::nil() { return None; }

        let now = Utc::now();
        let month_start = now.date_naive().with_day(1)?.and_hms_opt(0, 0, 0)?;

        let totals: Vec<BigDecimal> = billing_payments::table
            .filter(billing_payments::branch_id.eq(branch_id))
            .filter(billing_payments::status.eq("completed"))
            .filter(billing_payments::created_at.ge(month_start))
            .select(billing_payments::amount)
            .load(&mut conn)
            .ok()?;

        let sum: f64 = totals.iter().map(bd_to_f64).sum();
        Some(sum)
    })
    .await
    .ok()
    .flatten();

    Html(format_currency(result.unwrap_or(0.0), "USD"))
}
pub(super) async fn handle_overdue(State(state): State<Arc<BillingApiState>>, headers: HeaderMap) -> impl IntoResponse {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot;

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;
        let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
            .unwrap_or_else(|| crate::get_bot_context(&pool, &get_default_bot));
        if branch_id == Uuid::nil() { return None; }

        let totals: Vec<BigDecimal> = billing_invoices::table
            .filter(billing_invoices::branch_id.eq(branch_id))
            .filter(billing_invoices::status.eq("overdue"))
            .select(billing_invoices::amount_due)
            .load(&mut conn)
            .ok()?;

        let sum: f64 = totals.iter().map(bd_to_f64).sum();
        Some(sum)
    })
    .await
    .ok()
    .flatten();

    Html(format_currency(result.unwrap_or(0.0), "USD"))
}
pub(super) async fn handle_billing_search(
    State(state): State<Arc<BillingApiState>>,
    headers: HeaderMap,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let q = query.q.clone().unwrap_or_default();
    if q.is_empty() {
        return Html(String::new());
    }

    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot;
    let search_term = format!("%{}%", q);

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;
        let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
            .unwrap_or_else(|| crate::get_bot_context(&pool, &get_default_bot));
        if branch_id == Uuid::nil() { return None; }

        billing_invoices::table
            .filter(billing_invoices::branch_id.eq(branch_id))
            .filter(
                billing_invoices::invoice_number.ilike(&search_term)
                    .or(billing_invoices::customer_name.ilike(&search_term))
                    .or(billing_invoices::customer_email.ilike(&search_term))
            )
            .order(billing_invoices::created_at.desc())
            .limit(20)
            .select((
                billing_invoices::id,
                billing_invoices::invoice_number,
                billing_invoices::customer_name,
                billing_invoices::status,
                billing_invoices::total,
                billing_invoices::currency,
            ))
            .load::<(Uuid, String, Option<String>, Option<String>, Option<BigDecimal>, Option<String>)>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    match result {
        Some(items) if !items.is_empty() => {
            let mut html = String::new();
            for (id, number, customer, status, total, currency) in items {
                let total_str = format_currency(bd_to_f64(&total.unwrap_or_default()), &currency.as_deref().unwrap_or("USD"));

                html.push_str(&format!(
                    r##"<div class="search-result-item" hx-get="/api/billing/invoices/{id}" hx-target="#invoice-detail"><span class="result-number">{}</span><span class="result-customer">{}</span><span class="result-status">{}</span><span class="result-total">{}</span></div>"##,
                    html_escape(&number),
                    html_escape(customer.as_deref().unwrap_or("")),
                    html_escape(status.as_deref().unwrap_or("")),
                    total_str
                ));
            }
            Html(format!(r#"<div class="search-results">{html}</div>"#))
        }
        _ => Html(format!(
            r#"<div class="search-results-empty"><p>No results for "{}"</p></div>"#,
            html_escape(&q)
        )),
    }
}
