//! #1441 C4/C7 — pipeline summary cards, the kanban column fragments and the
//! reporting fragments (conversion funnel by source, weighted forecast).
//!
//! Every monetary card formats with the branch's dominant currency instead of
//! a hardcoded `$`, and the funnel/forecast fragments give the suite the
//! reporting the epic asks for without a second round trip to another app.

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::Html,
};
use diesel::dsl::sum;
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use super::lists::or_dash;
use super::{branch_ctx, format_money, GridQuery};
use crate::models::html_escape;
use crate::schema::crm_deals;
use crate::CrateState;

/// Stage probability fallback used when a deal carries none (the org-configured
/// stage weights live in `crm_pipeline_stages`; the built-ins mirror
/// `models::stage_probability`).
fn stage_weight(stage: &str) -> f64 {
    match stage {
        "new" => 0.10,
        "qualified" | "qualification" => 0.25,
        "proposal" => 0.50,
        "negotiation" => 0.75,
        "won" | "converted" => 1.0,
        _ => 0.0,
    }
}

/// The currency most used by the branch's open pipeline; falls back to USD so
/// a card never renders a bare number.
fn dominant_currency(conn: &mut diesel::PgConnection, branch_id: Uuid) -> String {
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        currency: Option<String>,
    }
    // The `uses` aggregate only ranks the rows (ORDER BY … DESC LIMIT 1), so it
    // is projected away rather than bound to an unread field.
    diesel::sql_query(
        "SELECT currency FROM crm_deals \
         WHERE branch_id = $1 AND currency IS NOT NULL \
         GROUP BY currency ORDER BY COUNT(*) DESC LIMIT 1",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch_id)
    .load::<Row>(conn)
    .ok()
    .and_then(|rows| rows.into_iter().next())
    .and_then(|r| r.currency)
    .unwrap_or_else(|| "USD".to_string())
}

/// One monetary card body, escaped and formatted in the branch currency.
fn money_card(total: Option<f64>, currency: &str) -> String {
    html_escape(&format_money(total.unwrap_or(0.0), Some(currency)))
}

/// `/api/crm/stats/pipeline-value` — sum of open deal values.
pub async fn handle_crm_stats_pipeline_value(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
) -> Html<String> {
    let Ok(mut conn) = state.db_pool.get() else {
        return Html(format_money(0.0, Some("USD")));
    };
    let branch_id = branch_ctx(&state, &headers, &mut conn);
    // `won <> true` is NULL (and therefore excluded) for the many rows that
    // never set the flag — the open pipeline reported €0.00 while deals were
    // open. `IS DISTINCT FROM` is NULL-safe: everything that is not explicitly
    // won counts as open.
    let total: Option<f64> = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .filter(crm_deals::won.is_distinct_from(true))
        .select(sum(crm_deals::value))
        .get_result(&mut conn)
        .unwrap_or(None);
    let currency = dominant_currency(&mut conn, branch_id);
    Html(money_card(total, &currency))
}

/// `/api/crm/stats/conversion-rate` — won deals over total deals.
pub async fn handle_crm_stats_conversion_rate(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
) -> Html<String> {
    let Ok(mut conn) = state.db_pool.get() else {
        return Html("0%".to_string());
    };
    let branch_id = branch_ctx(&state, &headers, &mut conn);
    let total: i64 = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);
    let won: i64 = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .filter(crm_deals::won.eq(true))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);
    let rate = if total > 0 { (won as f64 / total as f64) * 100.0 } else { 0.0 };
    Html(format!("{rate:.0}%"))
}

/// `/api/crm/stats/avg-deal` — average value across the branch's deals.
pub async fn handle_crm_stats_avg_deal(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
) -> Html<String> {
    let Ok(mut conn) = state.db_pool.get() else {
        return Html(format_money(0.0, Some("USD")));
    };
    let branch_id = branch_ctx(&state, &headers, &mut conn);
    let total: Option<f64> = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .select(sum(crm_deals::value))
        .get_result(&mut conn)
        .unwrap_or(None);
    let count: i64 = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);
    let avg = match (total, count) {
        (Some(t), c) if c > 0 => t / c as f64,
        _ => 0.0,
    };
    let currency = dominant_currency(&mut conn, branch_id);
    Html(money_card(Some(avg), &currency))
}

/// `/api/crm/stats/won-month` — value of deals won in the last 31 days.
pub async fn handle_crm_stats_won_month(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
) -> Html<String> {
    let Ok(mut conn) = state.db_pool.get() else {
        return Html(format_money(0.0, Some("USD")));
    };
    let branch_id = branch_ctx(&state, &headers, &mut conn);
    let total: Option<f64> = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .filter(crm_deals::won.eq(true))
        .filter(crm_deals::closed_at.ge(chrono::Utc::now() - chrono::Duration::days(31)))
        .select(sum(crm_deals::value))
        .get_result(&mut conn)
        .unwrap_or(None);
    let currency = dominant_currency(&mut conn, branch_id);
    Html(money_card(total, &currency))
}

/// `/api/ui/crm/stats/forecast` — weighted pipeline forecast (open pipeline ×
/// stage probability) with the open count and the currency it is in.
pub async fn handle_crm_stats_forecast(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
) -> Html<String> {
    let Ok(mut conn) = state.db_pool.get() else {
        return Html(r#"<div class="report-empty">Forecast unavailable</div>"#.to_string());
    };
    let branch_id = branch_ctx(&state, &headers, &mut conn);

    let rows: Vec<(Option<String>, Option<f64>)> = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .filter(crm_deals::closed_at.is_null())
        .select((crm_deals::stage, crm_deals::value))
        .load(&mut conn)
        .unwrap_or_default();

    let currency = dominant_currency(&mut conn, branch_id);
    let mut open_value = 0.0;
    let mut weighted = 0.0;
    let mut open_count = 0i64;
    for (stage, value) in &rows {
        let v = value.unwrap_or(0.0);
        open_value += v;
        weighted += v * stage_weight(stage.as_deref().unwrap_or("new"));
        open_count += 1;
    }
    let confidence = if open_count > 0 {
        (weighted / open_value.max(1.0) * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };
    Html(format!(
        r#"<div class="report-card"><div class="report-label">Weighted forecast</div><div class="report-value">{weighted}</div><div class="report-meta">{open_count} open · {open_value} pipeline · {confidence:.0}% confidence</div></div>"#,
        weighted = html_escape(&format_money(weighted, Some(&currency))),
        open_value = html_escape(&format_money(open_value, Some(&currency))),
    ))
}

/// `/api/ui/crm/stats/funnel` — conversion funnel by lead source: how many
/// leads each source produced and how many converted.
pub async fn handle_crm_stats_funnel(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
) -> Html<String> {
    let Ok(mut conn) = state.db_pool.get() else {
        return Html(r#"<div class="report-empty">Funnel unavailable</div>"#.to_string());
    };
    let branch_id = branch_ctx(&state, &headers, &mut conn);

    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        source: Option<String>,
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        total: i64,
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        won: i64,
    }
    let rows: Vec<Row> = diesel::sql_query(
        "SELECT source, COUNT(*) AS total, \
                COUNT(*) FILTER (WHERE won = true) AS won \
         FROM crm_deals WHERE branch_id = $1 \
         GROUP BY source ORDER BY total DESC",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch_id)
    .load(&mut conn)
    .unwrap_or_default();

    if rows.is_empty() {
        return Html(r#"<div class="report-empty">No funnel data yet</div>"#.to_string());
    }
    let currency = dominant_currency(&mut conn, branch_id);
    let mut html = String::from(r#"<div class="report-card"><div class="report-label">Funnel by source</div><table class="crm-table report-table"><thead><tr><th>Source</th><th>Leads</th><th>Won</th><th>Rate</th></tr></thead><tbody>"#);
    for row in rows {
        let rate = if row.total > 0 {
            (row.won as f64 / row.total as f64) * 100.0
        } else {
            0.0
        };
        html.push_str(&format!(
            "<tr><td>{source}</td><td>{total}</td><td>{won}</td><td>{rate:.0}%</td></tr>",
            source = or_dash(row.source.as_deref()),
            total = row.total,
            won = row.won,
        ));
    }
    html.push_str(&format!(
        r#"</tbody></table><div class="report-meta">Values in {currency}</div></div>"#,
        currency = html_escape(&currency)
    ));
    Html(html)
}

/// `/api/crm/count?stage=` — column badge count for the kanban.
pub async fn handle_crm_count(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Query(query): Query<GridQuery>,
) -> Html<String> {
    let Ok(mut conn) = state.db_pool.get() else {
        return Html("0".to_string());
    };
    let branch_id = branch_ctx(&state, &headers, &mut conn);
    let stage = query.stage.unwrap_or_else(|| "all".to_string());

    let count: i64 = if stage == "all" || stage.is_empty() {
        crm_deals::table
            .filter(crm_deals::branch_id.eq(branch_id))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0)
    } else {
        crm_deals::table
            .filter(crm_deals::branch_id.eq(branch_id))
            .filter(crm_deals::stage.eq(&stage))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0)
    };
    Html(count.to_string())
}

/// `/api/crm/pipeline?stage=` — the kanban column body.
pub async fn handle_crm_pipeline(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Query(query): Query<GridQuery>,
) -> Html<String> {
    let Ok(mut conn) = state.db_pool.get() else {
        return Html(r#"<div class="pipeline-empty"><p>No items yet</p></div>"#.to_string());
    };
    let branch_id = branch_ctx(&state, &headers, &mut conn);
    let stage = query.stage.unwrap_or_else(|| "new".to_string());

    let leads: Vec<crate::models::CrmDeal> = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .filter(crm_deals::stage.eq(&stage))
        .order(crm_deals::created_at.desc())
        .limit(20)
        .load(&mut conn)
        .unwrap_or_default();

    if leads.is_empty() {
        return Html(format!(
            r#"<div class="pipeline-empty"><p>No {} items yet</p></div>"#,
            html_escape(&stage)
        ));
    }

    let contact_ids: Vec<Uuid> = leads.iter().filter_map(|l| l.contact_id).collect();
    let contacts = super::lists::load_contacts(&mut conn, &contact_ids);
    let currency = dominant_currency(&mut conn, branch_id);

    let mut html = String::new();
    for lead in leads {
        let contact = super::lists::contact_name(contacts.iter().find(|c| Some(c.id) == lead.contact_id));
        let value = match lead.value {
            Some(v) => html_escape(&format_money(v, lead.currency.as_deref().or(Some(&currency)))),
            None => "-".to_string(),
        };
        html.push_str(&format!(
            r##"<div class="pipeline-card" draggable="true" data-id="{id}">
<div class="pipeline-card-header">
<span class="lead-title">{title}</span>
<span class="lead-value">{value}</span>
</div>
<div class="pipeline-card-body">
<span class="lead-contact">{contact}</span>
<span class="lead-probability">{probability}%</span>
</div>
<div class="pipeline-card-actions">
<button class="btn-sm" hx-put="/api/crm/leads/{id}/stage?stage=qualified" hx-swap="none">Qualify</button>
<button class="btn-sm btn-accent" hx-post="/api/crm/leads/{id}/convert" hx-swap="none">Convert</button>
<button class="btn-sm btn-secondary" data-action="view" data-entity="lead" data-id="{id}">View</button>
</div>
</div>"##,
            id = lead.id,
            title = or_dash(lead.title.as_deref().or(lead.name.as_deref())),
            value = value,
            contact = contact,
            probability = lead.probability.unwrap_or(0),
        ));
    }
    Html(html)
}
