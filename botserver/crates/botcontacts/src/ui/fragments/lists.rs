//! #1441 C2/B — shared helpers plus the Contacts and Accounts grids.
//!
//! Every grid in this tree answers the same contract: `?search=&stage=&sort=
//! &dir=&offset=` over the branch's rows, ranked in-process (bounded window)
//! so column headers are plain links and paging keeps the active filters. Rows
//! carry `data-id` plus View/Edit/Delete actions for the suite drawer.

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::Html,
};
use diesel::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use super::{
    branch_ctx, format_money, is_ascending, order_by, pager_bar, row_actions, GridQuery, PAGE_SIZE,
    SORT_WINDOW,
};
use crate::models::{html_escape, CrmAccount, CrmContact};
use crate::schema::{crm_accounts, crm_contacts, crm_deals};
use crate::schema_ext::users;
use crate::CrateState;

/// Cuts one page out of an already-ranked vector.
pub fn page<T>(rows: Vec<T>, offset: i64) -> Vec<T> {
    let start = offset.max(0) as usize;
    rows.into_iter().skip(start).take(PAGE_SIZE as usize).collect()
}

/// Non-empty trimmed value, or `"-"` — never renders a raw `None`.
pub fn or_dash(value: Option<&str>) -> String {
    match value.map(str::trim).filter(|v| !v.is_empty()) {
        Some(v) => html_escape(v),
        None => "-".to_string(),
    }
}

pub fn contact_name(contact: Option<&CrmContact>) -> String {
    match contact {
        Some(c) => {
            let full = full_name(c);
            if full.is_empty() {
                or_dash(c.email.as_deref())
            } else {
                html_escape(&full)
            }
        }
        None => "-".to_string(),
    }
}

pub fn account_name(account: Option<&CrmAccount>) -> String {
    match account {
        Some(a) => html_escape(&a.name),
        None => "-".to_string(),
    }
}

pub fn full_name(c: &CrmContact) -> String {
    format!(
        "{} {}",
        c.first_name.as_deref().unwrap_or(""),
        c.last_name.as_deref().unwrap_or("")
    )
    .trim()
    .to_string()
}

/// `users.id` → email, so `owner_id` renders as a name without one query per row.
pub fn owner_emails(
    conn: &mut diesel::PgConnection,
    owners: &[Uuid],
) -> HashMap<Uuid, String> {
    if owners.is_empty() {
        return HashMap::new();
    }
    users::table
        .filter(users::id.eq_any(owners.to_vec()))
        .select((users::id, users::email))
        .load::<(Uuid, String)>(conn)
        .unwrap_or_default()
        .into_iter()
        .collect()
}

pub fn load_contacts(conn: &mut diesel::PgConnection, ids: &[Uuid]) -> Vec<CrmContact> {
    if ids.is_empty() {
        return Vec::new();
    }
    crm_contacts::table
        .filter(crm_contacts::id.eq_any(ids.to_vec()))
        .load(conn)
        .unwrap_or_default()
}

pub fn load_accounts(conn: &mut diesel::PgConnection, ids: &[Uuid]) -> Vec<CrmAccount> {
    if ids.is_empty() {
        return Vec::new();
    }
    crm_accounts::table
        .filter(crm_accounts::id.eq_any(ids.to_vec()))
        .load(conn)
        .unwrap_or_default()
}

/// Rebuilds the active filters as a query suffix so pager links keep the
/// search/stage/sort the user is looking at.
pub fn filter_suffix(query: &GridQuery) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(stage) = query
        .stage
        .as_deref()
        .filter(|s| !s.is_empty() && *s != "all")
    {
        parts.push(format!("&stage={}", urlencode(stage)));
    }
    if let Some(search) = query.search.as_deref().filter(|s| !s.trim().is_empty()) {
        parts.push(format!("&search={}", urlencode(search)));
    }
    if let Some(sort) = query.sort.as_deref().filter(|s| !s.is_empty()) {
        parts.push(format!("&sort={}", urlencode(sort)));
        if let Some(dir) = query.dir.as_deref().filter(|d| !d.is_empty()) {
            parts.push(format!("&dir={}", urlencode(dir)));
        }
    }
    parts.join("")
}

pub fn urlencode(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "+".to_string(),
            other => other
                .to_string()
                .bytes()
                .map(|b| format!("%{b:02X}"))
                .collect::<String>(),
        })
        .collect()
}

/// `/api/ui/crm/contacts` — selection checkbox, identity, company/title,
/// e-mail, phone, owner and the row actions.
pub async fn handle_crm_contacts(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Query(query): Query<GridQuery>,
) -> Html<String> {
    let Ok(mut conn) = state.db_pool.get() else {
        return Html(r#"<tr><td colspan="8">Service unavailable</td></tr>"#.to_string());
    };
    let branch_id = branch_ctx(&state, &headers, &mut conn);

    let mut rows: Vec<CrmContact> = crm_contacts::table
        .filter(crm_contacts::branch_id.eq(branch_id))
        .order(crm_contacts::created_at.desc())
        .limit(SORT_WINDOW)
        .load(&mut conn)
        .unwrap_or_default();

    if let Some(term) = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let needle = term.to_lowercase();
        rows.retain(|c| {
            [
                c.first_name.as_deref(),
                c.last_name.as_deref(),
                c.email.as_deref(),
                c.company.as_deref(),
                c.job_title.as_deref(),
            ]
            .into_iter()
            .flatten()
            .any(|v| v.to_lowercase().contains(&needle))
        });
    }
    if let Some(owner) = query.owner_id {
        rows.retain(|c| c.owner_id == Some(owner));
    }

    let asc = is_ascending(query.dir.as_deref());
    match query.sort.as_deref().unwrap_or("created_at") {
        "name" => rows.sort_by(|a, b| {
            order_by(
                Some(&full_name(a).to_lowercase()),
                Some(&full_name(b).to_lowercase()),
                asc,
            )
        }),
        "email" => rows.sort_by(|a, b| order_by(a.email.as_deref(), b.email.as_deref(), asc)),
        "company" => rows.sort_by(|a, b| order_by(a.company.as_deref(), b.company.as_deref(), asc)),
        _ => rows.sort_by(|a, b| order_by(Some(&a.created_at), Some(&b.created_at), asc)),
    }

    let total = rows.len() as i64;
    let offset = query.offset.unwrap_or(0).max(0);
    let page_rows = page(rows, offset);

    let mut html = String::new();
    for c in page_rows {
        html.push_str(&format!(
            r#"<tr class="crm-row" data-id="{id}" data-entity="contact">
<td><input type="checkbox" class="opp-select" data-id="{id}"></td>
<td class="contact-name">{name}</td>
<td class="contact-company">{company}</td>
<td class="contact-title">{title}</td>
<td class="contact-email">{email}</td>
<td class="contact-phone">{phone}</td>
<td class="contact-owner">{owner}</td>
{actions}
</tr>"#,
            id = c.id,
            name = contact_name(Some(&c)),
            company = or_dash(c.company.as_deref()),
            title = or_dash(c.job_title.as_deref()),
            email = or_dash(c.email.as_deref()),
            phone = or_dash(c.phone.as_deref()),
            owner = or_dash(c.owner_id.map(|_| "assigned")),
            actions = row_actions("contact", c.id),
        ));
    }
    if html.is_empty() {
        return Html(r#"<tr><td colspan="8">No contacts yet</td></tr>"#.to_string());
    }
    html.push_str(&pager_bar("contacts", &filter_suffix(&query), total, offset));
    Html(html)
}

/// `/api/ui/crm/accounts` — accounts grid with the real deal count per
/// account (previously a hardcoded `-`).
pub async fn handle_crm_accounts(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Query(query): Query<GridQuery>,
) -> Html<String> {
    let Ok(mut conn) = state.db_pool.get() else {
        return Html(r#"<tr><td colspan="7">Service unavailable</td></tr>"#.to_string());
    };
    let branch_id = branch_ctx(&state, &headers, &mut conn);

    let mut rows: Vec<CrmAccount> = crm_accounts::table
        .filter(crm_accounts::branch_id.eq(branch_id))
        .order(crm_accounts::created_at.desc())
        .limit(SORT_WINDOW)
        .load(&mut conn)
        .unwrap_or_default();

    if let Some(term) = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let needle = term.to_lowercase();
        rows.retain(|a| {
            [a.name.as_str(), a.industry.as_deref().unwrap_or("")]
                .iter()
                .any(|v| v.to_lowercase().contains(&needle))
        });
    }

    let asc = is_ascending(query.dir.as_deref());
    match query.sort.as_deref().unwrap_or("created_at") {
        "name" => rows.sort_by(|a, b| {
            order_by(
                Some(&a.name.to_lowercase()),
                Some(&b.name.to_lowercase()),
                asc,
            )
        }),
        "industry" => rows.sort_by(|a, b| order_by(a.industry.as_deref(), b.industry.as_deref(), asc)),
        "revenue" => {
            rows.sort_by(|a, b| order_by(a.annual_revenue.as_ref(), b.annual_revenue.as_ref(), asc))
        }
        _ => rows.sort_by(|a, b| order_by(Some(&a.created_at), Some(&b.created_at), asc)),
    }

    let total = rows.len() as i64;
    let offset = query.offset.unwrap_or(0).max(0);
    let page_rows = page(rows, offset);

    // Deal counts for the accounts rendered on this page, in one round trip.
    let page_ids: Vec<Uuid> = page_rows.iter().map(|a| a.id).collect();
    let mut deal_counts: HashMap<Uuid, i64> = HashMap::new();
    if !page_ids.is_empty() {
        let grouped: Vec<(Option<Uuid>, i64)> = crm_deals::table
            .filter(crm_deals::branch_id.eq(branch_id))
            .filter(crm_deals::account_id.eq_any(&page_ids))
            .group_by(crm_deals::account_id)
            .select((crm_deals::account_id, diesel::dsl::count(crm_deals::id)))
            .load(&mut conn)
            .unwrap_or_default();
        for (account_id, count) in grouped {
            if let Some(id) = account_id {
                deal_counts.insert(id, count);
            }
        }
    }

    let mut html = String::new();
    for a in page_rows {
        let revenue = a
            .annual_revenue
            .map(|v| html_escape(&format_money(v, None)))
            .unwrap_or_else(|| "-".to_string());
        let deals = deal_counts.get(&a.id).copied().unwrap_or(0);
        html.push_str(&format!(
            r#"<tr class="crm-row" data-id="{id}" data-entity="account">
<td class="account-name">{name}</td>
<td class="account-industry">{industry}</td>
<td class="account-phone">{phone}</td>
<td class="account-city">{city}</td>
<td class="account-revenue">{revenue}</td>
<td class="account-contacts">{deals}</td>
{actions}
</tr>"#,
            id = a.id,
            name = html_escape(&a.name),
            industry = or_dash(a.industry.as_deref()),
            phone = or_dash(a.phone.as_deref()),
            city = or_dash(a.city.as_deref()),
            revenue = revenue,
            deals = deals,
            actions = row_actions("account", a.id),
        ));
    }
    if html.is_empty() {
        return Html(r#"<tr><td colspan="7">No accounts yet</td></tr>"#.to_string());
    }
    html.push_str(&pager_bar("accounts", &filter_suffix(&query), total, offset));
    Html(html)
}
