use super::*;

pub(super) fn count_rows(conn: &mut diesel::PgConnection, sql: &str) -> Vec<CountRow> {
    diesel::sql_query(sql)
        .load::<CountRow>(conn)
        .unwrap_or_default()
}
pub(super) fn money(value: Option<&bigdecimal::BigDecimal>) -> String {
    value
        .map(|v| format!("${v:.2}"))
        .unwrap_or_else(|| "$0.00".to_string())
}

// ── CRM reports ────────────────────────────────────────────────────────────
pub async fn crm_pipeline(
    State(state): State<Arc<ReportsState>>,
) -> Html<String> {
    let Ok(mut conn) = state.pool.get() else {
        return Html(String::new());
    };
    let branch = (state.default_bot)(&mut conn);

    let stages = ["new", "contacted", "qualified", "proposal", "negotiation", "won"];
    let colors = ["#4caf50", "#8bc34a", "#cddc39", "#ffc107", "#ff9800", "#2196f3"];
    let mut funnel = String::new();
    let mut max_count: i64 = 1;
    let mut counts = std::collections::HashMap::new();

    for stage in stages.iter() {
        let count: i64 = diesel::sql_query(
            "SELECT COUNT(*)::bigint AS count FROM crm_deals WHERE branch_id = $1 AND stage = $2",
        )
        .bind::<diesel::sql_types::Uuid, _>(branch)
        .bind::<diesel::sql_types::Text, _>(stage)
        .get_result(&mut conn)
        .map(|r: CountRow| r.count)
        .unwrap_or(0);
        counts.insert(stage, count);
        max_count = max_count.max(count);
    }

    for (i, stage) in stages.iter().enumerate() {
        let count = counts.get(stage).copied().unwrap_or(0);
        let pct = if max_count > 0 {
            (count as f64 / max_count as f64) * 100.0
        } else {
            0.0
        };
        let label = capitalize(stage);
        funnel.push_str(&format!(
            r#"<div class="funnel-stage" style="--width: {pct:.0}%; --color: {color}"><span class="stage-label">{label}</span><span class="stage-value">{count}</span></div>"#,
            color = colors[i],
        ));
    }

    Html(format!(
        r##"<div class="report-card-header"><h4>Sales Pipeline</h4><span class="report-badge">Funnel</span></div>
<div class="report-card-body"><div class="chart-placeholder"><div class="funnel-chart">{funnel}</div></div></div>"##
    ))
}
pub async fn crm_conversion(
    State(state): State<Arc<ReportsState>>,
) -> Html<String> {
    let Ok(mut conn) = state.pool.get() else {
        return Html(String::new());
    };
    let branch = (state.default_bot)(&mut conn);

    let total: i64 = diesel::sql_query(
        "SELECT COUNT(*)::bigint AS count FROM crm_deals WHERE branch_id = $1",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .get_result(&mut conn)
    .map(|r: CountRow| r.count)
    .unwrap_or(0);

    let won: i64 = diesel::sql_query(
        "SELECT COUNT(*)::bigint AS count FROM crm_deals WHERE branch_id = $1 AND won = true",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .get_result(&mut conn)
    .map(|r: CountRow| r.count)
    .unwrap_or(0);

    let leads: i64 = diesel::sql_query(
        "SELECT COUNT(*)::bigint AS count FROM crm_leads WHERE branch_id = $1",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .get_result(&mut conn)
    .map(|r: CountRow| r.count)
    .unwrap_or(0);

    let rate = if total > 0 { (won as f64 / total as f64) * 100.0 } else { 0.0 };

    Html(format!(
        r##"<div class="report-card-header"><h4>Lead Conversion Rate</h4><span class="report-badge">KPI</span></div>
<div class="report-card-body"><div class="kpi-display">
    <div class="kpi-main"><span class="kpi-value">{rate:.0}%</span></div>
    <div class="kpi-details">
        <div class="kpi-detail-item"><span class="detail-label">Leads</span><span class="detail-value">{leads}</span></div>
        <div class="kpi-detail-item"><span class="detail-label">Opportunities</span><span class="detail-value">{total}</span></div>
        <div class="kpi-detail-item"><span class="detail-label">Won</span><span class="detail-value">{won}</span></div>
    </div>
</div></div>"##
    ))
}
pub async fn crm_won_lost(
    State(state): State<Arc<ReportsState>>,
) -> Html<String> {
    let Ok(mut conn) = state.pool.get() else {
        return Html(String::new());
    };
    let branch = (state.default_bot)(&mut conn);

    let won: i64 = diesel::sql_query(
        "SELECT COUNT(*)::bigint AS count FROM crm_deals WHERE branch_id = $1 AND won = true",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .get_result(&mut conn)
    .map(|r: CountRow| r.count)
    .unwrap_or(0);

    let lost: i64 = diesel::sql_query(
        "SELECT COUNT(*)::bigint AS count FROM crm_deals WHERE branch_id = $1 AND won = false",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .get_result(&mut conn)
    .map(|r: CountRow| r.count)
    .unwrap_or(0);

    let total = won + lost;
    let won_pct = if total > 0 { (won as f64 / total as f64) * 100.0 } else { 0.0 };
    let lost_pct = if total > 0 { (lost as f64 / total as f64) * 100.0 } else { 0.0 };

    Html(format!(
        r##"<div class="report-card-header"><h4>Won/Lost Analysis</h4><span class="report-badge">Chart</span></div>
<div class="report-card-body"><div class="donut-chart-container">
    <div class="donut-chart">
        <svg viewBox="0 0 100 100" class="donut-svg">
            <circle cx="50" cy="50" r="40" fill="none" stroke="#4CAF50" stroke-width="15" stroke-dasharray="{won_pct:.0} 100" stroke-dashoffset="25" class="donut-segment won"/>
            <circle cx="50" cy="50" r="40" fill="none" stroke="#F44336" stroke-width="15" stroke-dasharray="{lost_pct:.0} 100" stroke-dashoffset="{won_pct_off:.0}" class="donut-segment lost"/>
        </svg>
        <div class="donut-center"><span class="donut-total">{total}</span><span class="donut-label">Total</span></div>
    </div>
    <div class="donut-legend">
        <div class="legend-item"><span class="legend-color" style="background:#4caf50"></span><span>Won</span><span class="legend-value">{won}</span></div>
        <div class="legend-item"><span class="legend-color" style="background:#f44336"></span><span>Lost</span><span class="legend-value">{lost}</span></div>
    </div>
</div></div>"##,
        won_pct_off = -won_pct - 25.0
    ))
}
pub async fn crm_forecast(
    State(state): State<Arc<ReportsState>>,
) -> Html<String> {
    let Ok(mut conn) = state.pool.get() else {
        return Html(String::new());
    };
    let branch = (state.default_bot)(&mut conn);

    let this_month: Option<bigdecimal::BigDecimal> = diesel::sql_query(
        "SELECT COALESCE(SUM(value), 0) AS value FROM crm_deals
         WHERE branch_id = $1 AND won = false AND expected_close_date BETWEEN date_trunc('month', CURRENT_DATE) AND (date_trunc('month', CURRENT_DATE) + INTERVAL '1 month')",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .get_result(&mut conn)
    .map(|r: ValueRow| r.value)
    .ok()
    .flatten();

    let next_month: Option<bigdecimal::BigDecimal> = diesel::sql_query(
        "SELECT COALESCE(SUM(value), 0) AS value FROM crm_deals
         WHERE branch_id = $1 AND won = false AND expected_close_date BETWEEN date_trunc('month', CURRENT_DATE) + INTERVAL '1 month' AND date_trunc('month', CURRENT_DATE) + INTERVAL '2 months'",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .get_result(&mut conn)
    .map(|r: ValueRow| r.value)
    .ok()
    .flatten();

    Html(format!(
        r##"<div class="report-card-header"><h4>Sales Forecast</h4><span class="report-badge">Projection</span></div>
<div class="report-card-body"><div class="forecast-display">
    <div class="forecast-main"><span class="forecast-label">This Month</span><span class="forecast-value">{this}</span></div>
    <div class="forecast-main secondary"><span class="forecast-label">Next Month</span><span class="forecast-value">{next}</span></div>
</div></div>"##,
        this = money(this_month.as_ref()),
        next = money(next_month.as_ref()),
    ))
}

// ── Billing reports ─────────────────────────────────────────────────────────
pub async fn billing_aging(
    State(state): State<Arc<ReportsState>>,
) -> Html<String> {
    let Ok(mut conn) = state.pool.get() else {
        return Html(String::new());
    };
    let branch = (state.default_bot)(&mut conn);

    let buckets = [
        ("Current", "due_date >= CURRENT_DATE"),
        ("1-30 days", "due_date BETWEEN CURRENT_DATE - INTERVAL '30 days' AND CURRENT_DATE - 1"),
        ("31-60 days", "due_date BETWEEN CURRENT_DATE - INTERVAL '60 days' AND CURRENT_DATE - INTERVAL '31 days'"),
        ("60+ days", "due_date < CURRENT_DATE - INTERVAL '60 days'"),
    ];

    let mut rows = String::new();
    for (label, cond) in buckets {
        let value: Option<bigdecimal::BigDecimal> = diesel::sql_query(format!(
            "SELECT COALESCE(SUM(amount_due), 0) AS value FROM billing_invoices WHERE branch_id = $1 AND status <> 'paid' AND status <> 'void' AND {cond}"
        ))
        .bind::<diesel::sql_types::Uuid, _>(branch)
        .get_result(&mut conn)
        .map(|r: ValueRow| r.value)
        .ok()
        .flatten();
        rows.push_str(&format!(
            r#"<div class="aging-row"><span class="aging-label">{label}</span><span class="aging-value">{value}</span></div>"#,
            value = money(value.as_ref())
        ));
    }

    Html(format!(
        r##"<div class="report-card-header"><h4>Billing Aging</h4><span class="report-badge">AR</span></div>
<div class="report-card-body"><div class="aging-list">{rows}</div></div>"##
    ))
}
pub async fn billing_monthly(
    State(state): State<Arc<ReportsState>>,
) -> Html<String> {
    let Ok(mut conn) = state.pool.get() else {
        return Html(String::new());
    };
    let branch = (state.default_bot)(&mut conn);

    let rows: Vec<CountRow> = count_rows(
        &mut conn,
        &format!(
            "SELECT to_char(paid_at, 'YYYY-MM') AS label, COUNT(*)::bigint AS count
             FROM billing_payments WHERE branch_id = '{}' AND paid_at IS NOT NULL AND status <> 'refunded'
             GROUP BY label ORDER BY label DESC LIMIT 6",
            branch
        ),
    );

    let mut bars = String::new();
    for row in rows.iter().rev() {
        let count = row.count;
        let label = &row.label;
        bars.push_str(&format!(
            r#"<div class="bar-column"><div class="bar-fill" style="height: {h}%"></div><span class="bar-label">{label}</span><span class="bar-value">{count}</span></div>"#,
            h = 20 + (count.min(100) * 70) / 100
        ));
    }

    Html(format!(
        r##"<div class="report-card-header"><h4>Monthly Payments</h4><span class="report-badge">Chart</span></div>
<div class="report-card-body"><div class="bar-chart">{bars}</div></div>"##
    ))
}
