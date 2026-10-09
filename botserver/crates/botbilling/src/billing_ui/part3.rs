use super::*;

pub(super) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
pub(super) fn format_currency(amount: f64, currency: &str) -> String {
    match currency.to_uppercase().as_str() {
        "USD" => format!("${:.2}", amount),
        "EUR" => format!("\u{20ac}{:.2}", amount),
        "GBP" => format!("\u{00a3}{:.2}", amount),
        "BRL" => format!("R${:.2}", amount),
        _ => format!("{:.2} {}", amount, currency),
    }
}
pub fn configure_billing_routes() -> Router<Arc<BillingApiState>> {
    Router::new()
        .route("/api/ui/billing/invoices", get(handle_invoices))
        .route("/api/ui/billing/payments", get(handle_payments))
        .route("/api/ui/billing/quotes", get(handle_quotes))
        .route("/api/ui/billing/stats/pending", get(handle_stats_pending))
        .route("/api/ui/billing/stats/revenue-month", get(handle_revenue_month))
        .route("/api/ui/billing/stats/paid-month", get(handle_paid_month))
        .route("/api/ui/billing/stats/overdue", get(handle_overdue))
        .route("/api/ui/billing/search", get(handle_billing_search))
        .route("/api/ui/billing/dashboard/metrics", get(handle_dashboard_metrics))
        .route("/api/ui/billing/dashboard/spending-chart", get(handle_spending_chart))
        .route("/api/ui/billing/dashboard/cost-breakdown", get(handle_cost_breakdown))
        .route("/api/ui/billing/dashboard/quotas", get(handle_dashboard_quotas))
        .route("/api/ui/billing/invoices/export", get(handle_invoices_export))
        .route("/api/ui/billing/subscription/upgrade", post(handle_subscription_upgrade))
        .route("/api/ui/billing/subscription/cancel", post(handle_subscription_cancel))
        .route("/api/admin/billing/quotas", put(handle_admin_billing_quotas))
        .route("/api/admin/billing/alerts", put(handle_admin_billing_alerts))
}
pub(super) async fn handle_dashboard_metrics(
    State(state): State<Arc<BillingApiState>>,
    headers: HeaderMap,
) -> Html<String> {
    let Ok(mut conn) = state.pool.get() else {
        return Html("Error connecting to DB".to_string());
    };

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| crate::get_bot_context(&state.pool, &state.get_default_bot));

    // 1. Current Period Spending (Sum of invoices total for branch_id)
    let total_spent = billing_invoices::table
        .filter(billing_invoices::branch_id.eq(branch_id))
        .select(diesel::dsl::sum(billing_invoices::total))
        .first::<Option<BigDecimal>>(&mut conn)
        .unwrap_or(None)
        .unwrap_or_else(|| BigDecimal::from(0));

    let total_spent_f = bd_to_f64(&total_spent);

    // 2. Projected (Total spent + sum of monthly recurring subscriptions amount)
    let total_recurring = billing_recurring::table
        .filter(billing_recurring::branch_id.eq(branch_id))
        .select(diesel::dsl::sum(billing_recurring::amount))
        .first::<Option<BigDecimal>>(&mut conn)
        .unwrap_or(None)
        .unwrap_or_else(|| BigDecimal::from(0));

    let total_recurring_f = bd_to_f64(&total_recurring);
    let projected_f = total_spent_f + total_recurring_f;

    // 3. Budget (Default Limit: $4000.00)
    let budget_limit = 4000.0;
    let budget_used_pct = if budget_limit <= 0.0 { 0.0 } else { (total_spent_f / budget_limit) * 100.0 };
    let budget_remaining = (budget_limit - total_spent_f).max(0.0);

    // 4. Savings (Sum of discount_amount for branch_id)
    let total_savings = billing_invoices::table
        .filter(billing_invoices::branch_id.eq(branch_id))
        .select(diesel::dsl::sum(billing_invoices::discount_amount))
        .first::<Option<BigDecimal>>(&mut conn)
        .unwrap_or(None)
        .unwrap_or_else(|| BigDecimal::from(0));

    let total_savings_f = bd_to_f64(&total_savings);

    let html = format!(
        r#"<div class="metric-card spending">
            <div class="metric-icon">
                <svg viewBox="0 0 24 24" width="24" height="24" stroke="currentColor" stroke-width="2" fill="none"><line x1="12" y1="1" x2="12" y2="23"></line><path d="M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6"></path></svg>
            </div>
            <div class="metric-content">
                <span class="metric-value">${:.2}</span>
                <span class="metric-label">Current Period</span>
            </div>
            <span class="metric-trend positive">-12% vs last period</span>
        </div>
        <div class="metric-card forecast">
            <div class="metric-icon">
                <svg viewBox="0 0 24 24" width="24" height="24" stroke="currentColor" stroke-width="2" fill="none"><polyline points="22 12 18 12 15 21 9 3 6 12 2 12"></polyline></svg>
            </div>
            <div class="metric-content">
                <span class="metric-value">${:.2}</span>
                <span class="metric-label">Projected</span>
            </div>
            <span class="metric-trend">End of period</span>
        </div>
        <div class="metric-card budget">
            <div class="metric-icon">
                <svg viewBox="0 0 24 24" width="24" height="24" stroke="currentColor" stroke-width="2" fill="none"><rect x="1" y="4" width="22" height="16" rx="2" ry="2"></rect><line x1="1" y1="10" x2="23" y2="10"></line></svg>
            </div>
            <div class="metric-content">
                <span class="metric-value">{:.1}%</span>
                <span class="metric-label">Budget Used</span>
            </div>
            <span class="metric-trend">${:.2} remaining</span>
        </div>
        <div class="metric-card savings">
            <div class="metric-icon">
                <svg viewBox="0 0 24 24" width="24" height="24" stroke="currentColor" stroke-width="2" fill="none"><path d="M19 5c-1.5 0-2.8 1.4-3 2-3.5-1.5-11-.3-11 5 0 1.8 0 3 2 4.5V20h4v-2h3v2h4v-4c1-.5 1.7-1 2-2h2v-4h-2c0-1-.5-1.5-1-2V5z"></path><path d="M2 9v1c0 1.1.9 2 2 2h1"></path></svg>
            </div>
            <div class="metric-content">
                <span class="metric-value">${:.2}</span>
                <span class="metric-label">Savings</span>
            </div>
            <span class="metric-trend positive">This month</span>
        </div>"#,
        total_spent_f, projected_f, budget_used_pct, budget_remaining, total_savings_f
    );
    Html(html)
}
pub(super) async fn handle_spending_chart(
    State(state): State<Arc<BillingApiState>>,
    headers: HeaderMap,
) -> Html<String> {
    let Ok(mut conn) = state.pool.get() else {
        return Html(String::new());
    };
    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| crate::get_bot_context(&state.pool, &state.get_default_bot));

    #[derive(diesel::QueryableByName)]
    struct BarRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        label: String,
        #[diesel(sql_type = diesel::sql_types::Numeric)]
        total: BigDecimal,
    }
    let rows: Vec<BarRow> = diesel::sql_query(
        "SELECT to_char(issue_date, 'Mon') AS label, COALESCE(SUM(total), 0) AS total \
         FROM billing_invoices WHERE branch_id = $1 \
         AND issue_date >= date_trunc('month', NOW()) - interval '6 months' \
         GROUP BY to_char(issue_date, 'Mon') ORDER BY MIN(issue_date)",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch_id)
    .load(&mut conn)
    .unwrap_or_default();

    if rows.is_empty() {
        return Html(r#"<div class="chart-empty">No invoice data yet</div>"#.to_string());
    }

    let max = rows.iter().map(|r| bd_to_f64(&r.total)).fold(0.0_f64, f64::max).max(1.0);
    let mut html = String::from(r#"<div class="chart-bars">"#);
    for r in &rows {
        let v = bd_to_f64(&r.total);
        let pct = (v / max * 100.0).max(4.0) as u32;
        html.push_str(&format!(
            r#"<div class="chart-bar" style="height: {pct}%"><span class="bar-label">{}</span><span class="bar-value">${:.0}</span></div>"#,
            html_escape(&r.label), v
        ));
    }
    html.push_str("</div>");
    Html(html)
}
pub(super) async fn handle_cost_breakdown(
    State(state): State<Arc<BillingApiState>>,
    headers: HeaderMap,
) -> Html<String> {
    let Ok(mut conn) = state.pool.get() else {
        return Html(String::new());
    };
    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| crate::get_bot_context(&state.pool, &state.get_default_bot));

    #[derive(diesel::QueryableByName)]
    struct StatusRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        status: String,
        #[diesel(sql_type = diesel::sql_types::Numeric)]
        total: BigDecimal,
    }
    let rows: Vec<StatusRow> = diesel::sql_query(
        "SELECT COALESCE(status, 'draft') AS status, COALESCE(SUM(total), 0) AS total \
         FROM billing_invoices WHERE branch_id = $1 GROUP BY COALESCE(status, 'draft')",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch_id)
    .load(&mut conn)
    .unwrap_or_default();

    if rows.is_empty() {
        return Html(r#"<div class="breakdown-empty">No invoice data yet</div>"#.to_string());
    }

    let grand: f64 = rows.iter().map(|r| bd_to_f64(&r.total)).sum();
    let colors = ["#3b82f6", "#10b981", "#f59e0b", "#8b5cf6", "#ef4444", "#06b6d4", "#84cc16"];
    let mut html = String::new();
    for (i, r) in rows.iter().enumerate() {
        let v = bd_to_f64(&r.total);
        let pct = if grand > 0.0 { v / grand * 100.0 } else { 0.0 };
        let color = colors[i % colors.len()];
        html.push_str(&format!(
            r#"<div class="breakdown-item"><div class="breakdown-color" style="background: {color}"></div><span class="breakdown-label">{}</span><span class="breakdown-value">${:.2}</span><span class="breakdown-percent">{:.0}%</span></div>"#,
            html_escape(&r.status), v, pct
        ));
    }
    Html(html)
}
pub(super) async fn handle_dashboard_quotas(
    State(state): State<Arc<BillingApiState>>,
    headers: HeaderMap,
) -> Html<String> {
    let Ok(mut conn) = state.pool.get() else {
        return Html("Error connecting to DB".to_string());
    };

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| crate::get_bot_context(&state.pool, &state.get_default_bot));

    // Dynamic counts
    let bot_count = diesel::sql_query(
        "SELECT COUNT(*)::bigint AS count FROM bots WHERE branch_id = $1"
    )
    .bind::<diesel::sql_types::Uuid, _>(branch_id)
    .get_result::<CountRow>(&mut conn)
    .map(|r| r.count)
    .unwrap_or(0);

    let contact_count = diesel::sql_query(
        "SELECT COUNT(*)::bigint AS count FROM crm_contacts WHERE org_id = $1"
    )
    .bind::<diesel::sql_types::Uuid, _>(branch_id)
    .get_result::<CountRow>(&mut conn)
    .map(|r| r.count)
    .unwrap_or(0);

    let invoice_count = diesel::sql_query(
        "SELECT COUNT(*)::bigint AS count FROM billing_invoices WHERE branch_id = $1"
    )
    .bind::<diesel::sql_types::Uuid, _>(branch_id)
    .get_result::<CountRow>(&mut conn)
    .map(|r| r.count)
    .unwrap_or(0);

    #[derive(diesel::QueryableByName)]
    struct QuotaRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        quota_key: String,
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        quota_limit: i64,
    }
    let quota_rows: Vec<QuotaRow> = diesel::sql_query(
        "SELECT quota_key, quota_limit FROM billing_quota_settings WHERE branch_id = $1",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch_id)
    .load(&mut conn)
    .unwrap_or_default();

    let mut quota_limits: HashMap<String, i64> = HashMap::new();
    for &(key, default) in DEFAULT_QUOTA_LIMITS {
        quota_limits.insert(key.to_string(), default);
    }
    for row in quota_rows {
        quota_limits.insert(row.quota_key, row.quota_limit);
    }

    let invoice_limit = quota_limits.get("invoices").copied().unwrap_or(100);
    let team_limit = quota_limits.get("team").copied().unwrap_or(50);
    let bot_limit = quota_limits.get("bots").copied().unwrap_or(10);

    let bot_pct = (bot_count as f64 / bot_limit as f64 * 100.0).min(100.0);
    let team_pct = (contact_count as f64 / team_limit as f64 * 100.0).min(100.0);
    let invoice_pct = (invoice_count as f64 / invoice_limit as f64 * 100.0).min(100.0);

    let html = format!(
        r#"<div class="quota-item"><div class="quota-header"><span class="quota-name">Invoices</span><span class="quota-usage">{invoice_count} / {invoice_limit}</span></div><div class="quota-bar"><div class="quota-fill" style="width: {invoice_pct:.1}%"></div></div></div>
        <div class="quota-item"><div class="quota-header"><span class="quota-name">Team Members</span><span class="quota-usage">{} / {}</span></div><div class="quota-bar"><div class="quota-fill" style="width: {:.1}%"></div></div></div>
        <div class="quota-item"><div class="quota-header"><span class="quota-name">Bots</span><span class="quota-usage">{} / {}</span></div><div class="quota-bar"><div class="quota-fill" style="width: {:.1}%"></div></div></div>"#,
        contact_count, team_limit, team_pct, bot_count, bot_limit, bot_pct
    );
    Html(html)
}
