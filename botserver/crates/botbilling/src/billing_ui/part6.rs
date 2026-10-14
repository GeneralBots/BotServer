use super::*;

pub(super) async fn handle_subscription_cancel(
    State(state): State<Arc<BillingApiState>>,
    headers: HeaderMap,
    Json(req): Json<CancelRequest>,
) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Json(serde_json::json!({
            "success": false,
            "error": "Database unavailable"
        }));
    };

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| crate::get_bot_context(&state.pool, &state.get_default_bot));

    match diesel::sql_query(
        "UPDATE billing_recurring \
         SET status = 'cancelled', cancelled_at = NOW(), updated_at = NOW() \
         WHERE branch_id = $1",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch_id)
    .execute(&mut conn)
    {
        Ok(rows) if rows > 0 => Json(serde_json::json!({
            "success": true,
            "message": "Subscription cancelled",
            "reason": req.reason
        })),
        Ok(_) => Json(serde_json::json!({
            "success": false,
            "error": "No active subscription found for this branch"
        })),
        Err(e) => {
            tracing::error!("Failed to cancel subscription: {e}");
            Json(serde_json::json!({
                "success": false,
                "error": "Failed to cancel subscription"
            }))
        }
    }
}
pub(super) async fn handle_admin_billing_quotas(
    State(state): State<Arc<BillingApiState>>,
    headers: HeaderMap,
    Json(quotas): Json<serde_json::Value>,
) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Json(serde_json::json!({
            "success": false,
            "error": "Database unavailable"
        }));
    };

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| crate::get_bot_context(&state.pool, &state.get_default_bot));

    let now = chrono::Utc::now();
    let mut saved: usize = 0;

    if let Some(obj) = quotas.as_object() {
        for (key, value) in obj {
            let Some(limit) = parse_quota_limit(value) else {
                continue;
            };
            let key = key.trim().to_lowercase();
            if key.is_empty() || key.len() > 64 {
                continue;
            }
            let result = diesel::sql_query(
                "INSERT INTO billing_quota_settings \
                 (id, branch_id, quota_key, quota_limit, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, $5) \
                 ON CONFLICT (branch_id, quota_key) \
                 DO UPDATE SET quota_limit = EXCLUDED.quota_limit, updated_at = $5",
            )
            .bind::<diesel::sql_types::Uuid, _>(Uuid::new_v4())
            .bind::<diesel::sql_types::Uuid, _>(branch_id)
            .bind::<diesel::sql_types::Text, _>(&key)
            .bind::<diesel::sql_types::BigInt, _>(limit)
            .bind::<diesel::sql_types::Timestamptz, _>(now)
            .execute(&mut conn);

            if result.is_ok() {
                saved += 1;
            }
        }
    }

    Json(serde_json::json!({
        "success": saved > 0,
        "saved": saved,
        "message": format!("Quotas updated ({saved} values saved)")
    }))
}
pub(super) async fn handle_admin_billing_alerts(
    State(state): State<Arc<BillingApiState>>,
    headers: HeaderMap,
    Json(settings): Json<serde_json::Value>,
) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Json(serde_json::json!({
            "success": false,
            "error": "Database unavailable"
        }));
    };

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| crate::get_bot_context(&state.pool, &state.get_default_bot));

    let settings_json = serde_json::to_string(&settings).unwrap_or_else(|_| "{}".to_string());
    let now = chrono::Utc::now();

    let result = diesel::sql_query(
        "INSERT INTO billing_alert_settings (id, branch_id, settings, created_at, updated_at) \
         VALUES ($1, $2, $3::jsonb, $4, $4) \
         ON CONFLICT (branch_id) DO UPDATE SET settings = EXCLUDED.settings, updated_at = $4",
    )
    .bind::<diesel::sql_types::Uuid, _>(Uuid::new_v4())
    .bind::<diesel::sql_types::Uuid, _>(branch_id)
    .bind::<diesel::sql_types::Text, _>(&settings_json)
    .bind::<diesel::sql_types::Timestamptz, _>(now)
    .execute(&mut conn);

    match result {
        Ok(_) => Json(serde_json::json!({
            "success": true,
            "message": "Alert settings updated successfully",
            "settings": settings
        })),
        Err(e) => {
            tracing::error!("Failed to persist billing alert settings: {e}");
            Json(serde_json::json!({
                "success": false,
                "error": "Failed to save alert settings"
            }))
        }
    }
}
pub(super) async fn handle_invoices(
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

        let mut db_query = billing_invoices::table
            .filter(billing_invoices::branch_id.eq(branch_id))
            .into_boxed();

        if let Some(ref status) = query.status {
            db_query = db_query.filter(billing_invoices::status.eq(status));
        }

        db_query = db_query.order(billing_invoices::created_at.desc());

        let limit = query.limit.unwrap_or(50);
        db_query = db_query.limit(limit);

        db_query
            .select((
                billing_invoices::id,
                billing_invoices::invoice_number,
                billing_invoices::customer_name,
                billing_invoices::customer_email,
                billing_invoices::status,
                billing_invoices::issue_date,
                billing_invoices::due_date,
                billing_invoices::total,
                billing_invoices::amount_due,
                billing_invoices::currency,
            ))
            .load::<(Uuid, String, Option<String>, Option<String>, Option<String>, NaiveDate, Option<NaiveDate>, Option<BigDecimal>, BigDecimal, Option<String>)>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    match result {
        Some(invoices) if !invoices.is_empty() => {
            let mut html = String::new();
            for (id, number, customer_name, customer_email, status, issue_date, due_date, total, amount_due, currency) in invoices {
                let name = customer_email.or(customer_name).unwrap_or_else(|| "Unknown".to_string());
                let total_str = format_currency(bd_to_f64(&total.unwrap_or_default()), currency.as_deref().unwrap_or("USD"));
                let due_str = format_currency(bd_to_f64(&amount_due), currency.as_deref().unwrap_or("USD"));
                let issue_str = issue_date.format("%Y-%m-%d").to_string();
                let due_date_str = due_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "-".to_string());

                let status_class = match status.as_deref().unwrap_or("draft") {
                    "paid" => "status-paid",
                    "sent" => "status-sent",
                    "overdue" => "status-overdue",
                    "void" => "status-void",
                    _ => "status-draft",
                };

                html.push_str(&format!(
                    r##"<tr class="invoice-row" data-id="{id}"><td class="invoice-number">{}</td><td class="invoice-customer">{}</td><td class="invoice-date">{}</td><td class="invoice-due">{}</td><td class="invoice-total">{}</td><td class="invoice-balance">{}</td><td class="invoice-status"><span class="{}">{}</span></td><td class="invoice-actions"><button class="btn-sm" hx-get="/api/billing/invoices/{id}" hx-target="#invoice-detail">View</button></td></tr>"##,
                    html_escape(&number),
                    html_escape(&name),
                    issue_str,
                    due_date_str,
                    total_str,
                    due_str,
                    status_class,
                    html_escape(status.as_deref().unwrap_or(""))
                ));
            }
            Html(html)
        }
        _ => Html(r#"<tr class="empty-row"><td colspan="8" class="empty-state"><div class="empty-icon">Document</div><p>No invoices yet</p><p class="empty-hint">Create your first invoice to get started</p></td></tr>"#.to_string()),
    }
}
pub(super) async fn handle_payments(
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

        let mut db_query = billing_payments::table
            .filter(billing_payments::branch_id.eq(branch_id))
            .into_boxed();

        if let Some(ref status) = query.status {
            db_query = db_query.filter(billing_payments::status.eq(status));
        }

        db_query = db_query.order(billing_payments::created_at.desc());

        let limit = query.limit.unwrap_or(50);
        db_query = db_query.limit(limit);

        db_query
            .select((
                billing_payments::id,
                billing_payments::payment_number,
                billing_payments::amount,
                billing_payments::currency,
                billing_payments::payment_method,
                billing_payments::payer_name,
                billing_payments::payer_email,
                billing_payments::status,
                billing_payments::paid_at,
            ))
            .load::<(Uuid, String, BigDecimal, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<DateTime<Utc>>)>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    match result {
        Some(payments) if !payments.is_empty() => {
            let mut html = String::new();
            for (id, number, amount, currency, method, payer_name, payer_email, status, paid_at) in payments {
                let amount_str = format_currency(bd_to_f64(&amount), currency.as_deref().unwrap_or("USD"));
                let payer = payer_name.or(payer_email).unwrap_or_else(|| "Unknown".to_string());
                let date_str = paid_at.map(|d| d.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_else(|| "-".to_string());

                let status_class = match status.as_deref().unwrap_or("draft") {
                    "completed" => "status-completed",
                    "pending" => "status-pending",
                    "refunded" => "status-refunded",
                    "failed" => "status-failed",
                    _ => "status-default",
                };

                html.push_str(&format!(
                    r##"<tr class="payment-row" data-id="{id}"><td class="payment-number">{}</td><td class="payment-payer">{}</td><td class="payment-amount">{}</td><td class="payment-method">{}</td><td class="payment-date">{}</td><td class="payment-status"><span class="{}">{}</span></td><td class="payment-actions"><button class="btn-sm" hx-get="/api/billing/payments/{id}" hx-target="#payment-detail">View</button></td></tr>"##,
                    html_escape(&number),
                    html_escape(&payer),
                    amount_str,
                    html_escape(method.as_deref().unwrap_or("")),
                    date_str,
                    status_class,
                    html_escape(status.as_deref().unwrap_or(""))
                ));
            }
            Html(html)
        }
        _ => Html(r#"<tr class="empty-row"><td colspan="7" class="empty-state"><div class="empty-icon">Payment</div><p>No payments recorded</p><p class="empty-hint">Payments will appear here when invoices are paid</p></td></tr>"#.to_string()),
    }
}
