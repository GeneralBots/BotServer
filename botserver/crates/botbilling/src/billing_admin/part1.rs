use super::*;

pub(super) fn bd_to_f64(bd: &BigDecimal) -> f64 {
    bd.to_f64().unwrap_or(0.0)
}
pub(super) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
pub(super) fn branch_id(state: &BillingApiState) -> Uuid {
    crate::get_bot_context(&state.pool, &state.get_default_bot)
}
pub fn configure_admin_billing_routes() -> Router<Arc<BillingApiState>> {
    Router::new()
        .route("/api/billing/usage", get(handle_usage))
        .route("/api/billing/payment-methods", get(handle_payment_methods))
        .route("/api/billing/address", get(handle_address))
        .route("/api/ui/billing/invoices/admin", get(handle_invoices))
        .route("/api/billing/retention-offers", get(handle_retention_offers))
        .route("/api/billing/upgrade", post(handle_upgrade))
        .route("/api/billing/accept-offer", post(handle_accept_offer))
        .route("/api/billing/pause", post(handle_pause))
        .route("/api/billing/cancel", post(handle_cancel))
        .route("/api/billing/subscription/upgrade", post(handle_subscription_upgrade))
        .route("/api/billing/subscription/cancel", post(handle_subscription_cancel))
        .route("/api/billing/invoices/export", get(handle_invoices_export))
        .route("/api/billing/invoices/unpaid", get(handle_invoices_unpaid))
        .route("/api/ui/billing/dashboard/alerts", get(handle_dashboard_alerts))
        .route("/api/ui/billing/dashboard/bot-usage", get(handle_dashboard_bot_usage))
        .route("/api/ui/billing/dashboard/recent-invoices", get(handle_dashboard_recent_invoices))
}
pub async fn handle_usage(State(state): State<Arc<BillingApiState>>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Html(String::new());
    };
    let branch = branch_id(&state);

    let messages: i64 = diesel::sql_query(
        "SELECT COUNT(*)::bigint AS count FROM message_history WHERE created_at > NOW() - INTERVAL '30 days'",
    )
    .get_result::<CountRow>(&mut conn)
    .map(|r| r.count)
    .unwrap_or(0);

    let storage: i64 = diesel::sql_query(
        "SELECT COALESCE(SUM(file_size), 0)::bigint AS count FROM kb_documents",
    )
    .get_result::<CountRow>(&mut conn)
    .map(|r| r.count)
    .unwrap_or(0);

    let api_calls: i64 = diesel::sql_query(
        "SELECT COUNT(*)::bigint AS count FROM message_history WHERE created_at > NOW() - INTERVAL '24 hours'",
    )
    .get_result::<CountRow>(&mut conn)
    .map(|r| r.count)
    .unwrap_or(0);

    let storage_gb = storage as f64 / 1_073_741_824.0;

    Html(format!(
        r#"<div class="usage-items">
    <div class="usage-item"><span class="usage-label">Messages (30d)</span><span class="usage-value">{messages}</span></div>
    <div class="usage-item"><span class="usage-label">Storage</span><span class="usage-value">{storage_gb:.1} GB</span></div>
    <div class="usage-item"><span class="usage-label">API Calls (24h)</span><span class="usage-value">{api_calls}</span></div>
    <div class="usage-item"><span class="usage-label">Bots</span><span class="usage-value">{bots}</span></div>
</div>"#,
        bots = {
            let b: i64 = diesel::sql_query(
                "SELECT COUNT(*)::bigint AS count FROM bots WHERE branch_id = $1",
            )
            .bind::<diesel::sql_types::Uuid, _>(branch)
            .get_result::<CountRow>(&mut conn)
            .map(|r| r.count)
            .unwrap_or(0);
            b
        },
    ))
}
pub async fn handle_payment_methods(State(state): State<Arc<BillingApiState>>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Html(String::new());
    };
    let branch = branch_id(&state);

    #[derive(diesel::QueryableByName)]
    struct MethodRow {
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        method: Option<String>,
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        count: i64,
    }

    let methods: Vec<MethodRow> = diesel::sql_query(
        "SELECT COALESCE(payment_method, 'unknown') AS method, COUNT(*)::bigint AS count
         FROM billing_payments WHERE branch_id = $1 GROUP BY method ORDER BY count DESC LIMIT 5",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .unwrap_or_default();

    if methods.is_empty() {
        return Html(
            r#"<div class="payment-methods"><div class="payment-method"><span class="method-icon">💳</span><span class="method-label">No payment method on file</span></div></div>"#
                .to_string(),
        );
    }

    let mut html = String::new();
    for m in methods {
        let label = m.method.unwrap_or_else(|| "—".to_string());
        html.push_str(&format!(
            r#"<div class="payment-method"><span class="method-icon">💳</span><span class="method-label">{label}</span><span class="method-meta">{count} payment(s)</span></div>"#,
            count = m.count,
        ));
    }
    Html(html)
}
pub async fn handle_address(State(state): State<Arc<BillingApiState>>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Html(String::new());
    };

    #[derive(diesel::QueryableByName)]
    struct AddressRow {
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        address: Option<String>,
        #[diesel(sql_type = diesel::sql_types::Varchar)]
        name: String,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
        email: Option<String>,
    }

    let row: Option<AddressRow> = diesel::sql_query(
        "SELECT customer_address AS address, customer_name AS name, customer_email AS email
         FROM billing_recurring ORDER BY created_at DESC LIMIT 1",
    )
    .get_result(&mut conn)
    .ok();

    match row {
        Some(r) => Html(format!(
            r#"<div class="address-card">
    <div class="address-line">{name}</div>
    <div class="address-line">{email}</div>
    <div class="address-line">{address}</div>
</div>"#,
            name = html_escape(&r.name),
            email = html_escape(r.email.as_deref().unwrap_or("")),
            address = html_escape(r.address.as_deref().unwrap_or("No billing address on file.")),
        )),
        None => Html(
            r#"<div class="address-card"><div class="address-line">No billing address on file.</div></div>"#.to_string(),
        ),
    }
}
pub async fn handle_invoices(State(state): State<Arc<BillingApiState>>) -> impl IntoResponse {
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
         WHERE branch_id = $1 ORDER BY created_at DESC LIMIT 12",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .unwrap_or_default();

    if rows.is_empty() {
        return Html(r#"<tr><td colspan="4" class="empty-cell">No invoices yet</td></tr>"#.to_string());
    }

    let mut html = String::new();
    for r in rows {
        html.push_str(&format!(
            r#"<tr>
    <td>#{number}</td>
    <td>{date}</td>
    <td>{total}</td>
    <td><span class="status-badge {status}">{status}</span></td>
</tr>"#,
            number = html_escape(&r.number),
            date = r.created_at.format("%b %d, %Y"),
            total = format_currency(r.total.as_ref().map(bd_to_f64).unwrap_or(0.0)),
            status = r.status.unwrap_or_else(|| "unknown".to_string()),
        ));
    }
    Html(html)
}
pub async fn handle_retention_offers(State(state): State<Arc<BillingApiState>>) -> impl IntoResponse {
    let _ = state;
    Html(
        r#"<div class="retention-offers">
    <div class="retention-offer">
        <div class="offer-info"><span class="offer-name">Annual discount</span><span class="offer-desc">Switch to yearly billing and save 25%</span></div>
        <button class="btn-secondary btn-sm" hx-post="/api/billing/accept-offer" hx-vals='{"offer": "discount_25"}' hx-swap="none">Apply</button>
    </div>
    <div class="retention-offer">
        <div class="offer-info"><span class="offer-name">Pause subscription</span><span class="offer-desc">Keep your data and pause billing for 30 days</span></div>
        <button class="btn-secondary btn-sm" hx-post="/api/billing/pause" hx-swap="none">Pause</button>
    </div>
</div>"#
            .to_string(),
    )
}
