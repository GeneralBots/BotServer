use super::*;

pub async fn handle_subscription_upgrade(State(state): State<Arc<BillingApiState>>, Json(req): Json<SubscriptionRequest>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "success": false, "error": "Database unavailable" })));
    };
    let branch = branch_id(&state);
    let plan = req.plan_id.unwrap_or_else(|| "shared".to_string());

    let _ = diesel::sql_query(
        "UPDATE billing_recurring SET plan_name = $1, status = 'active', updated_at = NOW() WHERE branch_id = $2",
    )
    .bind::<diesel::sql_types::Text, _>(&plan)
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .execute(&mut conn);

    (
        StatusCode::OK,
        Json(serde_json::json!({ "success": true, "plan": plan })),
    )
}
pub async fn handle_subscription_cancel(State(state): State<Arc<BillingApiState>>, Json(req): Json<SubscriptionRequest>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "success": false, "error": "Database unavailable" })));
    };
    let branch = branch_id(&state);
    let reason = req.reason.unwrap_or_default();

    let _ = diesel::sql_query(
        "UPDATE billing_recurring SET status = 'cancelled', cancelled_at = NOW(), updated_at = NOW() WHERE branch_id = $1",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .execute(&mut conn);

    let _ = reason;
    (
        StatusCode::OK,
        Json(serde_json::json!({ "success": true })),
    )
}
pub async fn handle_invoices_export(State(state): State<Arc<BillingApiState>>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Database unavailable").into_response();
    };
    let branch = branch_id(&state);

    #[derive(diesel::QueryableByName)]
    struct ExportRow {
        #[diesel(sql_type = diesel::sql_types::Varchar)]
        number: String,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
        customer_name: Option<String>,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
        total: Option<BigDecimal>,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
        status: Option<String>,
        #[diesel(sql_type = diesel::sql_types::Date)]
        issue_date: chrono::NaiveDate,
    }

    let rows: Vec<ExportRow> = diesel::sql_query(
        "SELECT invoice_number AS number, customer_name, total, status, issue_date FROM billing_invoices WHERE branch_id = $1 ORDER BY issue_date DESC",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .unwrap_or_default();

    let mut csv = String::from("invoice_number,customer,amount,status,issue_date\n");
    for r in rows {
        let amount = r.total.as_ref().map(bd_to_f64).unwrap_or(0.0);
        csv.push_str(&format!(
            "{},{},{:.2},{},{}\n",
            r.number,
            r.customer_name.unwrap_or_default().replace(',', " "),
            amount,
            r.status.unwrap_or_else(|| "unknown".to_string()),
            r.issue_date,
        ));
    }

    (
        [
            (axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"invoices.csv\"",
            ),
        ],
        csv,
    )
        .into_response()
}
pub async fn handle_invoices_unpaid(State(state): State<Arc<BillingApiState>>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Html(r#"<option value="">Unavailable</option>"#.to_string());
    };
    let branch = branch_id(&state);

    #[derive(diesel::QueryableByName)]
    struct InvRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Varchar)]
        number: String,
    }

    let rows: Vec<InvRow> = diesel::sql_query(
        "SELECT id, invoice_number AS number FROM billing_invoices WHERE branch_id = $1 AND (status = 'sent' OR status = 'overdue' OR status IS NULL) ORDER BY created_at DESC LIMIT 50",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .unwrap_or_default();

    if rows.is_empty() {
        return Html(r#"<option value="">No unpaid invoices</option>"#.to_string());
    }

    let mut html = String::new();
    for r in rows {
        html.push_str(&format!(
            r#"<option value="{id}">#{number}</option>"#,
            id = r.id,
            number = html_escape(&r.number),
        ));
    }
    Html(html)
}
pub async fn handle_dashboard_alerts(State(state): State<Arc<BillingApiState>>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Html(String::new());
    };
    let branch = branch_id(&state);

    let overdue: i64 = diesel::sql_query(
        "SELECT COUNT(*)::bigint AS count FROM billing_invoices WHERE branch_id = $1 AND status <> 'paid' AND status <> 'void' AND due_date < CURRENT_DATE",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .get_result::<CountRow>(&mut conn)
    .map(|r| r.count)
    .unwrap_or(0);

    let paused: i64 = diesel::sql_query(
        "SELECT COUNT(*)::bigint AS count FROM billing_recurring WHERE branch_id = $1 AND status = 'paused'",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .get_result::<CountRow>(&mut conn)
    .map(|r| r.count)
    .unwrap_or(0);

    if overdue == 0 && paused == 0 {
        return Html(r#"<div class="alerts-empty">No billing alerts</div>"#.to_string());
    }

    let mut html = String::new();
    if overdue > 0 {
        html.push_str(&format!(
            r#"<div class="alert-item warning"><span class="alert-icon">⚠️</span><span class="alert-text">{overdue} invoice(s) overdue</span></div>"#
        ));
    }
    if paused > 0 {
        html.push_str(
            r#"<div class="alert-item info"><span class="alert-icon">⏸</span><span class="alert-text">Subscription is paused</span></div>"#,
        );
    }
    Html(html)
}
pub async fn handle_dashboard_bot_usage(State(state): State<Arc<BillingApiState>>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Html(String::new());
    };
    let branch = branch_id(&state);

    #[derive(diesel::QueryableByName)]
    struct BotRow {
        #[diesel(sql_type = diesel::sql_types::Varchar)]
        name: String,
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        count: i64,
    }

    let rows: Vec<BotRow> = diesel::sql_query(
        "SELECT b.name, COUNT(mh.id)::bigint AS count
         FROM bots b LEFT JOIN user_sessions us ON us.bot_id = b.bot_id
         LEFT JOIN message_history mh ON mh.session_id = us.id
         WHERE b.branch_id = $1 GROUP BY b.id, b.name ORDER BY count DESC LIMIT 5",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .unwrap_or_default();

    if rows.is_empty() {
        return Html(r#"<div class="bots-empty">No bot usage data</div>"#.to_string());
    }

    let max = rows.iter().map(|r| r.count).max().unwrap_or(1).max(1);
    let mut html = String::new();
    for r in rows {
        let pct = (r.count as f64 / max as f64) * 100.0;
        html.push_str(&format!(
            r#"<div class="usage-bar-row"><span class="bar-label">{name}</span><div class="bar-track"><div class="bar-fill" style="width:{pct:.0}%"></div></div><span class="bar-value">{count}</span></div>"#,
            name = html_escape(&r.name),
            count = r.count,
        ));
    }
    Html(html)
}
pub async fn handle_dashboard_recent_invoices(State(state): State<Arc<BillingApiState>>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Html(String::new());
    };
    let branch = branch_id(&state);

    #[derive(diesel::QueryableByName)]
    struct InvRow {
        #[diesel(sql_type = diesel::sql_types::Varchar)]
        number: String,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
        total: Option<BigDecimal>,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
        status: Option<String>,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)]
        created_at: chrono::DateTime<Utc>,
    }

    let rows: Vec<InvRow> = diesel::sql_query(
        "SELECT invoice_number AS number, total, status, created_at FROM billing_invoices
         WHERE branch_id = $1 ORDER BY created_at DESC LIMIT 5",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .unwrap_or_default();

    if rows.is_empty() {
        return Html(r#"<div class="empty-state"><p>No recent invoices</p></div>"#.to_string());
    }

    let mut html = String::new();
    for r in rows {
        html.push_str(&format!(
            r#"<div class="invoice-row"><span class="invoice-number">#{number}</span><span class="invoice-date">{date}</span><span class="invoice-total">{total}</span><span class="invoice-status {status}">{status}</span></div>"#,
            number = html_escape(&r.number),
            date = r.created_at.format("%b %d"),
            total = format_currency(r.total.as_ref().map(bd_to_f64).unwrap_or(0.0)),
            status = r.status.unwrap_or_else(|| "unknown".to_string()),
        ));
    }
    Html(html)
}
pub(super) fn format_currency(amount: f64) -> String {
    format!("${amount:.2}")
}
