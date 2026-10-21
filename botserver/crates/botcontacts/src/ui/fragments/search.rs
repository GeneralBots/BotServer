//! #1441 — global CRM search across deals, contacts and accounts, plus the
//! `<option>` lists the create/edit forms use for their relationship pickers.

use axum::{extract::{Query, State}, http::HeaderMap, response::Html};
use diesel::prelude::*;
use std::sync::Arc;

use super::lists::full_name;
use super::{branch_ctx, format_money, TermQuery};
use crate::models::{html_escape, CrmDeal};
use crate::schema::{crm_accounts, crm_contacts, crm_deals};
use crate::CrateState;

/// `/api/crm/search?q=` — the header dropdown: deals, contacts and accounts.
pub async fn handle_crm_search(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Query(query): Query<TermQuery>,
) -> Html<String> {
    let Ok(mut conn) = state.db_pool.get() else {
        return Html(r#"<div class="search-empty"><p>Search unavailable</p></div>"#.to_string());
    };
    let branch_id = branch_ctx(&state, &headers, &mut conn);
    let q = query.q.unwrap_or_default().trim().to_lowercase();
    if q.is_empty() {
        return Html(
            r#"<div class="search-empty"><p>Type to search deals, contacts and accounts</p></div>"#
                .to_string(),
        );
    }
    let pattern = format!("%{q}%");

    let deals: Vec<CrmDeal> = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .filter(crm_deals::title.like(&pattern).or(crm_deals::name.like(&pattern)))
        .order(crm_deals::updated_at.desc())
        .limit(8)
        .load(&mut conn)
        .unwrap_or_default();

    let contacts: Vec<crate::models::CrmContact> = crm_contacts::table
        .filter(crm_contacts::branch_id.eq(branch_id))
        .filter(
            crm_contacts::first_name
                .like(&pattern)
                .or(crm_contacts::last_name.like(&pattern))
                .or(crm_contacts::email.like(&pattern)),
        )
        .order(crm_contacts::updated_at.desc())
        .limit(8)
        .load(&mut conn)
        .unwrap_or_default();

    let accounts: Vec<crate::models::CrmAccount> = crm_accounts::table
        .filter(crm_accounts::branch_id.eq(branch_id))
        .filter(crm_accounts::name.like(&pattern))
        .order(crm_accounts::updated_at.desc())
        .limit(8)
        .load(&mut conn)
        .unwrap_or_default();

    if deals.is_empty() && contacts.is_empty() && accounts.is_empty() {
        return Html(format!(
            r#"<div class="search-empty"><p>No results for "{q}"</p></div>"#
        ));
    }

    let mut html = String::new();
    if !deals.is_empty() {
        html.push_str(r#"<div class="search-group"><div class="search-group-title">Deals</div>"#);
        for deal in deals {
            let title = deal.title.as_deref().or(deal.name.as_deref()).unwrap_or("");
            let meta = deal
                .value
                .map(|v| format_money(v, deal.currency.as_deref()))
                .unwrap_or_default();
            html.push_str(&result_link("lead", deal.id, title, &meta));
        }
        html.push_str("</div>");
    }
    if !contacts.is_empty() {
        html.push_str(r#"<div class="search-group"><div class="search-group-title">Contacts</div>"#);
        for contact in contacts {
            html.push_str(&result_link(
                "contact",
                contact.id,
                &full_name(&contact),
                contact.email.as_deref().unwrap_or(""),
            ));
        }
        html.push_str("</div>");
    }
    if !accounts.is_empty() {
        html.push_str(r#"<div class="search-group"><div class="search-group-title">Accounts</div>"#);
        for account in accounts {
            html.push_str(&result_link(
                "account",
                account.id,
                &account.name,
                account.industry.as_deref().unwrap_or(""),
            ));
        }
        html.push_str("</div>");
    }
    Html(html)
}

/// Search hits open the record drawer in-place (no full navigation) so the
/// user keeps their grid state.
fn result_link(entity: &str, id: uuid::Uuid, name: &str, meta: &str) -> String {
    format!(
        r##"<a class="search-result" href="#" data-action="view" data-entity="{entity}" data-id="{id}"><span class="search-result-name">{label}</span><span class="search-result-meta">{detail}</span></a>"##,
        entity = entity,
        id = id,
        label = html_escape(name),
        detail = html_escape(meta),
    )
}

/// `/api/crm/accounts/search?q=` — account `<option>` list for form selects.
pub async fn handle_crm_accounts_search(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Query(query): Query<TermQuery>,
) -> Html<String> {
    let Ok(mut conn) = state.db_pool.get() else {
        return Html(r#"<option value="">No accounts</option>"#.to_string());
    };
    let branch_id = branch_ctx(&state, &headers, &mut conn);

    let mut accounts: Vec<crate::models::CrmAccount> = crm_accounts::table
        .filter(crm_accounts::branch_id.eq(branch_id))
        .order(crm_accounts::name)
        .limit(200)
        .load(&mut conn)
        .unwrap_or_default();

    if let Some(term) = query.q.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let needle = term.to_lowercase();
        accounts.retain(|a| a.name.to_lowercase().contains(&needle));
    }
    if accounts.is_empty() {
        return Html(r#"<option value="">No accounts available</option>"#.to_string());
    }

    let mut html = String::new();
    for account in accounts {
        html.push_str(&format!(
            r#"<option value="{id}">{name}</option>"#,
            id = account.id,
            name = html_escape(&account.name)
        ));
    }
    Html(html)
}

/// `/api/crm/contacts/search?q=` — contact `<option>` list for form selects.
pub async fn handle_crm_contacts_search(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Query(query): Query<TermQuery>,
) -> Html<String> {
    let Ok(mut conn) = state.db_pool.get() else {
        return Html(r#"<option value="">No contacts</option>"#.to_string());
    };
    let branch_id = branch_ctx(&state, &headers, &mut conn);

    let mut contacts: Vec<crate::models::CrmContact> = crm_contacts::table
        .filter(crm_contacts::branch_id.eq(branch_id))
        .order(crm_contacts::created_at.desc())
        .limit(200)
        .load(&mut conn)
        .unwrap_or_default();

    if let Some(term) = query.q.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let needle = term.to_lowercase();
        contacts.retain(|c| {
            [full_name(c), c.email.clone().unwrap_or_default()]
                .iter()
                .any(|v| v.to_lowercase().contains(&needle))
        });
    }
    if contacts.is_empty() {
        return Html(r#"<option value="">No contacts available</option>"#.to_string());
    }

    let mut html = String::new();
    for contact in contacts {
        let label = format!(
            "{} · {}",
            full_name(&contact),
            contact.email.clone().unwrap_or_default()
        );
        html.push_str(&format!(
            r#"<option value="{id}">{label}</option>"#,
            id = contact.id,
            label = html_escape(&label)
        ));
    }
    Html(html)
}

/// `/api/crm/opportunities/search?q=` — open-opportunity `<option>` list.
pub async fn handle_crm_opportunities_search(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Query(query): Query<TermQuery>,
) -> Html<String> {
    let Ok(mut conn) = state.db_pool.get() else {
        return Html(r#"<option value="">No opportunities</option>"#.to_string());
    };
    let branch_id = branch_ctx(&state, &headers, &mut conn);

    let mut opportunities: Vec<CrmDeal> = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .filter(crm_deals::lead_id.is_not_null())
        .order(crm_deals::created_at.desc())
        .limit(200)
        .load(&mut conn)
        .unwrap_or_default();

    if let Some(term) = query.q.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let needle = term.to_lowercase();
        opportunities.retain(|o| {
            [o.title.as_deref().unwrap_or(""), o.name.as_deref().unwrap_or("")]
                .iter()
                .any(|v| v.to_lowercase().contains(&needle))
        });
    }
    if opportunities.is_empty() {
        return Html(r#"<option value="">No open opportunities</option>"#.to_string());
    }

    let mut html = String::new();
    for opp in opportunities {
        let label = opp.title.as_deref().or(opp.name.as_deref()).unwrap_or("");
        html.push_str(&format!(
            r#"<option value="{id}">{label}</option>"#,
            id = opp.id,
            label = html_escape(label)
        ));
    }
    Html(html)
}
