//! #1441 C2/A3 — the pipeline list fragments: `/api/ui/crm/deals` (the lead
//! board's list twin) and `/api/ui/crm/opportunities`.
//!
//! Opportunities are the converted subset of `crm_deals` (`lead_id` set by
//! `POST /api/crm/leads/:id/convert`), so the grid reads the same rows the
//! Convert endpoint writes. The old implementation read `crm_opportunities`,
//! a parallel table nothing writes to — Convert dead-ended in the UI.

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::Html,
};
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use super::lists::{account_name, contact_name, filter_suffix, load_accounts, load_contacts,
    or_dash, owner_emails, page};
use super::{
    branch_ctx, format_money, is_ascending, order_by, pager_bar, row_actions, GridQuery, SORT_WINDOW,
};
use crate::models::{html_escape, CrmDeal};
use crate::schema::crm_deals;
use crate::CrateState;

fn title_of(d: &CrmDeal) -> Option<&str> {
    d.title.as_deref().or(d.name.as_deref())
}

fn money(value: Option<f64>, currency: Option<&str>) -> String {
    match value {
        Some(v) => html_escape(&format_money(v, currency)),
        None => "-".to_string(),
    }
}

/// `/api/ui/crm/deals` — title, currency-aware value, stage, resolved
/// contact/account, probability, close date, owner and row actions.
pub async fn handle_crm_deals(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Query(query): Query<GridQuery>,
) -> Html<String> {
    let Ok(mut conn) = state.db_pool.get() else {
        return Html(r#"<tr><td colspan="10">Service unavailable</td></tr>"#.to_string());
    };
    let branch_id = branch_ctx(&state, &headers, &mut conn);

    let mut rows: Vec<CrmDeal> = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .order(crm_deals::created_at.desc())
        .limit(SORT_WINDOW)
        .load(&mut conn)
        .unwrap_or_default();

    if let Some(stage) = query.stage.as_deref().filter(|s| !s.is_empty() && *s != "all") {
        rows.retain(|d| d.stage.as_deref() == Some(stage));
    }
    if let Some(term) = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let needle = term.to_lowercase();
        rows.retain(|d| {
            [title_of(d), d.description.as_deref()]
                .into_iter()
                .flatten()
                .any(|v| v.to_lowercase().contains(&needle))
        });
    }
    if let Some(owner) = query.owner_id {
        rows.retain(|d| d.owner_id == Some(owner));
    }

    let asc = is_ascending(query.dir.as_deref());
    match query.sort.as_deref().unwrap_or("created_at") {
        "title" => rows.sort_by(|a, b| order_by(title_of(a), title_of(b), asc)),
        "value" => rows.sort_by(|a, b| order_by(a.value.as_ref(), b.value.as_ref(), asc)),
        "stage" => rows.sort_by(|a, b| order_by(a.stage.as_deref(), b.stage.as_deref(), asc)),
        "close" => rows.sort_by(|a, b| {
            order_by(
                a.expected_close_date.as_ref(),
                b.expected_close_date.as_ref(),
                asc,
            )
        }),
        _ => rows.sort_by(|a, b| order_by(Some(&a.created_at), Some(&b.created_at), asc)),
    }

    let total = rows.len() as i64;
    let offset = query.offset.unwrap_or(0).max(0);
    let owners = owner_emails(&mut conn, &ownable(&rows));
    let page_rows = page(rows, offset);

    let contact_ids: Vec<Uuid> = page_rows.iter().filter_map(|d| d.contact_id).collect();
    let account_ids: Vec<Uuid> = page_rows.iter().filter_map(|d| d.account_id).collect();
    let contacts = load_contacts(&mut conn, &contact_ids);
    let accounts = load_accounts(&mut conn, &account_ids);

    let mut html = String::new();
    for d in &page_rows {
        let close = d
            .expected_close_date
            .map(|c| c.to_string())
            .unwrap_or_else(|| "-".to_string());
        let owner = owner_label(d, &owners);
        html.push_str(&format!(
            r#"<tr class="crm-row" data-id="{id}" data-entity="lead">
<td><input type="checkbox" class="opp-select" data-id="{id}"></td>
<td class="deal-title">{title}</td>
<td class="deal-value">{value}</td>
<td class="deal-stage">{stage}</td>
<td class="deal-contact">{contact}</td>
<td class="deal-account">{account}</td>
<td class="deal-probability">{probability}</td>
<td class="deal-close">{close}</td>
<td class="deal-owner">{owner}</td>
{actions}
</tr>"#,
            id = d.id,
            title = or_dash(title_of(d)),
            value = money(d.value, d.currency.as_deref()),
            stage = or_dash(d.stage.as_deref()),
            contact = contact_name(contacts.iter().find(|c| Some(c.id) == d.contact_id)),
            account = account_name(accounts.iter().find(|a| Some(a.id) == d.account_id)),
            probability = d
                .probability
                .map(|p| format!("{p}%"))
                .unwrap_or_else(|| "-".to_string()),
            close = html_escape(&close),
            owner = owner,
            actions = row_actions("lead", d.id),
        ));
    }
    if html.is_empty() {
        return Html(r#"<tr><td colspan="10">No deals yet</td></tr>"#.to_string());
    }
    html.push_str(&pager_bar("deals", &filter_suffix(&query), total, offset));
    Html(html)
}

/// `/api/ui/crm/opportunities` — the converted rows, with the close actions
/// (`/api/crm/opportunities/:id/close`) reachable from the row.
pub async fn handle_crm_opportunities(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Query(query): Query<GridQuery>,
) -> Html<String> {
    let Ok(mut conn) = state.db_pool.get() else {
        return Html(r#"<tr><td colspan="10">Service unavailable</td></tr>"#.to_string());
    };
    let branch_id = branch_ctx(&state, &headers, &mut conn);

    let mut rows: Vec<CrmDeal> = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .filter(crm_deals::lead_id.is_not_null())
        .order(crm_deals::created_at.desc())
        .limit(SORT_WINDOW)
        .load(&mut conn)
        .unwrap_or_default();

    if let Some(stage) = query.stage.as_deref().filter(|s| !s.is_empty() && *s != "all") {
        rows.retain(|d| d.stage.as_deref() == Some(stage));
    }
    if let Some(term) = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let needle = term.to_lowercase();
        rows.retain(|d| {
            [title_of(d)]
                .into_iter()
                .flatten()
                .any(|v| v.to_lowercase().contains(&needle))
        });
    }

    let asc = is_ascending(query.dir.as_deref());
    match query.sort.as_deref().unwrap_or("created_at") {
        "name" => rows.sort_by(|a, b| order_by(title_of(a), title_of(b), asc)),
        "value" => rows.sort_by(|a, b| order_by(a.value.as_ref(), b.value.as_ref(), asc)),
        "stage" => rows.sort_by(|a, b| order_by(a.stage.as_deref(), b.stage.as_deref(), asc)),
        "close" => rows.sort_by(|a, b| {
            order_by(
                a.expected_close_date.as_ref(),
                b.expected_close_date.as_ref(),
                asc,
            )
        }),
        _ => rows.sort_by(|a, b| order_by(Some(&a.created_at), Some(&b.created_at), asc)),
    }

    let total = rows.len() as i64;
    let offset = query.offset.unwrap_or(0).max(0);
    let owners = owner_emails(&mut conn, &ownable(&rows));
    let page_rows = page(rows, offset);

    let mut html = String::new();
    for d in &page_rows {
        let close = d
            .expected_close_date
            .map(|c| c.to_string())
            .unwrap_or_else(|| "-".to_string());
        let status = match d.won {
            Some(true) => "Won",
            Some(false) => "Lost",
            None => "Open",
        };
        html.push_str(&format!(
            r#"<tr class="crm-row" data-id="{id}" data-opp-id="{id}" data-entity="opportunity">
<td><input type="checkbox" class="opp-select" data-id="{id}"></td>
<td class="opp-name">{name}</td>
<td class="opp-value">{value}</td>
<td class="opp-stage">{stage}</td>
<td class="opp-probability">{probability}</td>
<td class="opp-close">{close}</td>
<td class="opp-source">{source}</td>
<td class="opp-status">{status}</td>
<td class="opp-owner">{owner}</td>
{actions}
</tr>"#,
            id = d.id,
            name = or_dash(title_of(d)),
            value = money(d.value, d.currency.as_deref()),
            stage = or_dash(d.stage.as_deref()),
            probability = d
                .probability
                .map(|p| format!("{p}%"))
                .unwrap_or_else(|| "-".to_string()),
            close = html_escape(&close),
            source = or_dash(d.source.as_deref()),
            status = status,
            owner = owner_label(d, &owners),
            actions = row_actions("opportunity", d.id),
        ));
    }
    if html.is_empty() {
        return Html(
            r#"<tr><td colspan="10">No opportunities yet — convert a qualified lead to create one</td></tr>"#
                .to_string(),
        );
    }
    html.push_str(&pager_bar(
        "opportunities",
        &filter_suffix(&query),
        total,
        offset,
    ));
    Html(html)
}

fn ownable(rows: &[CrmDeal]) -> Vec<Uuid> {
    let mut owners: Vec<Uuid> = rows.iter().filter_map(|r| r.owner_id).collect();
    owners.sort();
    owners.dedup();
    owners
}

fn owner_label(d: &CrmDeal, owners: &std::collections::HashMap<Uuid, String>) -> String {
    d.owner_id
        .and_then(|id| owners.get(&id).cloned())
        .map(|email| html_escape(&email))
        .unwrap_or_else(|| "-".to_string())
}
