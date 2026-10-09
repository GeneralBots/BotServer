use super::*;

pub async fn billing_payments(
    State(state): State<Arc<ReportsState>>,
) -> Html<String> {
    let Ok(mut conn) = state.pool.get() else {
        return Html(String::new());
    };
    let branch = (state.default_bot)(&mut conn);

    #[derive(Debug, diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct PaymentRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        payer: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        method: String,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
        amount: Option<bigdecimal::BigDecimal>,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)]
        paid_at: chrono::DateTime<Utc>,
    }

    let payments: Vec<PaymentRow> = diesel::sql_query(
        "SELECT COALESCE(payer_name, payer_email, '—') AS payer,
                COALESCE(payment_method, '—') AS method, amount, paid_at
         FROM billing_payments WHERE branch_id = $1 AND paid_at IS NOT NULL
         ORDER BY paid_at DESC LIMIT 6",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .unwrap_or_default();

    let mut rows = String::new();
    for p in payments {
        rows.push_str(&format!(
            r#"<tr><td>{payer}</td><td>{method}</td><td>{amount}</td><td>{date}</td></tr>"#,
            payer = p.payer,
            method = p.method,
            amount = money(p.amount.as_ref()),
            date = p.paid_at.format("%b %d, %Y"),
        ));
    }

    Html(format!(
        r##"<div class="report-card-header"><h4>Recent Payments</h4><span class="report-badge">Table</span></div>
<div class="report-card-body"><table class="report-table"><thead><tr><th>Payer</th><th>Method</th><th>Amount</th><th>Date</th></tr></thead><tbody>{rows}</tbody></table></div>"##
    ))
}
pub async fn billing_revenue(
    State(state): State<Arc<ReportsState>>,
) -> Html<String> {
    let Ok(mut conn) = state.pool.get() else {
        return Html(String::new());
    };
    let branch = (state.default_bot)(&mut conn);

    let total: Option<bigdecimal::BigDecimal> = diesel::sql_query(
        "SELECT COALESCE(SUM(amount), 0) AS value FROM billing_payments
         WHERE branch_id = $1 AND paid_at IS NOT NULL AND status <> 'refunded'",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .get_result(&mut conn)
    .map(|r: ValueRow| r.value)
    .ok()
    .flatten();

    let month: Option<bigdecimal::BigDecimal> = diesel::sql_query(
        "SELECT COALESCE(SUM(amount), 0) AS value FROM billing_payments
         WHERE branch_id = $1 AND paid_at IS NOT NULL AND paid_at >= date_trunc('month', CURRENT_DATE)",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .get_result(&mut conn)
    .map(|r: ValueRow| r.value)
    .ok()
    .flatten();

    let outstanding: Option<bigdecimal::BigDecimal> = diesel::sql_query(
        "SELECT COALESCE(SUM(amount_due), 0) AS value FROM billing_invoices
         WHERE branch_id = $1 AND status <> 'paid' AND status <> 'void'",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .get_result(&mut conn)
    .map(|r: ValueRow| r.value)
    .ok()
    .flatten();

    Html(format!(
        r##"<div class="report-card-header"><h4>Revenue</h4><span class="report-badge">KPI</span></div>
<div class="report-card-body"><div class="kpi-display">
    <div class="kpi-main"><span class="kpi-value">{total}</span></div>
    <div class="kpi-details">
        <div class="kpi-detail-item"><span class="detail-label">This Month</span><span class="detail-value">{month}</span></div>
        <div class="kpi-detail-item"><span class="detail-label">Outstanding</span><span class="detail-value">{outstanding}</span></div>
    </div>
</div></div>"##,
        total = money(total.as_ref()),
        month = money(month.as_ref()),
        outstanding = money(outstanding.as_ref()),
    ))
}

// ── Support reports ─────────────────────────────────────────────────────────
pub async fn support_ai_rate(
    State(state): State<Arc<ReportsState>>,
) -> Html<String> {
    let Ok(mut conn) = state.pool.get() else {
        return Html(String::new());
    };
    let branch = (state.default_bot)(&mut conn);

    let total: i64 = diesel::sql_query(
        "SELECT COUNT(*)::bigint AS count FROM support_tickets WHERE branch_id = $1",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .get_result(&mut conn)
    .map(|r: CountRow| r.count)
    .unwrap_or(0);

    let ai: i64 = diesel::sql_query(
        "SELECT COUNT(*)::bigint AS count FROM support_tickets WHERE branch_id = $1 AND source = 'ai'",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .get_result(&mut conn)
    .map(|r: CountRow| r.count)
    .unwrap_or(0);

    let rate = if total > 0 { (ai as f64 / total as f64) * 100.0 } else { 0.0 };

    Html(format!(
        r##"<div class="report-card-header"><h4>AI Resolution Rate</h4><span class="report-badge">KPI</span></div>
<div class="report-card-body"><div class="kpi-display">
    <div class="kpi-main"><span class="kpi-value">{rate:.0}%</span></div>
    <div class="kpi-details">
        <div class="kpi-detail-item"><span class="detail-label">AI Resolved</span><span class="detail-value">{ai}</span></div>
        <div class="kpi-detail-item"><span class="detail-label">Total</span><span class="detail-value">{total}</span></div>
    </div>
</div></div>"##
    ))
}
pub async fn support_by_category(
    State(state): State<Arc<ReportsState>>,
) -> Html<String> {
    let Ok(mut conn) = state.pool.get() else {
        return Html(String::new());
    };
    let branch = (state.default_bot)(&mut conn);

    let rows: Vec<CountRow> = count_rows(
        &mut conn,
        &format!(
            "SELECT COALESCE(category, 'Uncategorized') AS label, COUNT(*)::bigint AS count
             FROM support_tickets WHERE branch_id = '{branch}' GROUP BY label ORDER BY count DESC LIMIT 6"
        ),
    );
    let max = rows.iter().map(|r| r.count).max().unwrap_or(1).max(1);

    let mut bars = String::new();
    for r in rows {
        let pct = (r.count as f64 / max as f64) * 100.0;
        bars.push_str(&format!(
            r#"<div class="bar-row"><span class="bar-label">{label}</span><div class="bar-track"><div class="bar-fill" style="width:{pct:.0}%"></div></div><span class="bar-value">{count}</span></div>"#,
            label = r.label,
            count = r.count,
        ));
    }

    Html(format!(
        r##"<div class="report-card-header"><h4>Tickets by Category</h4><span class="report-badge">Chart</span></div>
<div class="report-card-body"><div class="horizontal-bars">{bars}</div></div>"##
    ))
}
pub async fn support_by_priority(
    State(state): State<Arc<ReportsState>>,
) -> Html<String> {
    let Ok(mut conn) = state.pool.get() else {
        return Html(String::new());
    };
    let branch = (state.default_bot)(&mut conn);

    let rows: Vec<CountRow> = count_rows(
        &mut conn,
        &format!(
            "SELECT COALESCE(priority, 'unknown') AS label, COUNT(*)::bigint AS count
             FROM support_tickets WHERE branch_id = '{branch}' GROUP BY label ORDER BY count DESC LIMIT 5"
        ),
    );

    let mut items = String::new();
    for r in rows {
        let color = match r.label.as_str() {
            "critical" | "urgent" => "#ef4444",
            "high" => "#f59e0b",
            "medium" | "normal" => "#3b82f6",
            _ => "#6b7280",
        };
        items.push_str(&format!(
            r#"<div class="priority-item"><span class="priority-dot" style="background:{color}"></span><span class="priority-label">{label}</span><span class="priority-count">{count}</span></div>"#,
            label = r.label,
            count = r.count,
        ));
    }

    Html(format!(
        r##"<div class="report-card-header"><h4>Tickets by Priority</h4><span class="report-badge">Chart</span></div>
<div class="report-card-body"><div class="priority-list">{items}</div></div>"##
    ))
}
pub async fn support_resolution_time(
    State(state): State<Arc<ReportsState>>,
) -> Html<String> {
    let Ok(mut conn) = state.pool.get() else {
        return Html(String::new());
    };
    let branch = (state.default_bot)(&mut conn);

    let avg: Option<f64> = diesel::sql_query(
        "SELECT AVG(EXTRACT(EPOCH FROM (resolved_at - created_at)) / 3600.0) AS avg
         FROM support_tickets WHERE branch_id = $1 AND resolved_at IS NOT NULL",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .get_result(&mut conn)
    .map(|r: AvgRow| r.avg)
    .ok()
    .flatten();

    let display = match avg {
        Some(h) if h < 24.0 => format!("{h:.1}h"),
        Some(h) => format!("{:.1}d", h / 24.0),
        None => "—".to_string(),
    };

    Html(format!(
        r##"<div class="report-card-header"><h4>Avg Resolution Time</h4><span class="report-badge">KPI</span></div>
<div class="report-card-body"><div class="kpi-display"><div class="kpi-main"><span class="kpi-value">{display}</span></div></div></div>"##
    ))
}
pub(super) fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
pub fn configure_reports_routes() -> axum::Router<Arc<ReportsState>> {
    use axum::routing::get;
    axum::Router::new()
        .route("/api/reports/crm/pipeline", get(crm_pipeline))
        .route("/api/reports/crm/conversion", get(crm_conversion))
        .route("/api/reports/crm/won-lost", get(crm_won_lost))
        .route("/api/reports/crm/forecast", get(crm_forecast))
        .route("/api/reports/billing/aging", get(billing_aging))
        .route("/api/reports/billing/monthly", get(billing_monthly))
        .route("/api/reports/billing/payments", get(billing_payments))
        .route("/api/reports/billing/revenue", get(billing_revenue))
        .route("/api/reports/support/ai-rate", get(support_ai_rate))
        .route("/api/reports/support/by-category", get(support_by_category))
        .route("/api/reports/support/by-priority", get(support_by_priority))
        .route("/api/reports/support/resolution-time", get(support_resolution_time))
}
