//! #1441 C1/A5 — record detail fragments for Contact, Account, Deal/Lead and
//! Opportunity: editable field grid, related records and the activity timeline
//! rendered from `crm_activities`.
//!
//! One endpoint serves every entity so the suite drawer opens any row type
//! (`/api/ui/crm/records/{entity}/{id}`); unknown entities render an explicit
//! error instead of an empty panel.

use axum::{extract::{Path, State}, http::HeaderMap, response::Html};
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use super::lists::full_name;
use super::{branch_ctx, format_money};
use crate::models::{html_escape, CrmActivity, CrmAccount, CrmContact, CrmDeal};
use crate::schema::{crm_accounts, crm_activities, crm_contacts, crm_deals};
use crate::CrateState;

/// Which related-record slot an activity belongs to.
fn activity_matches(a: &CrmActivity, entity: &str, id: Uuid) -> bool {
    match entity {
        "contact" => a.contact_id == Some(id),
        "account" => a.account_id == Some(id),
        "opportunity" => a.opportunity_id == Some(id),
        _ => a.lead_id == Some(id) || a.contact_id == Some(id),
    }
}

fn field(label: &str, value: &str) -> String {
    format!(
        r#"<div class="detail-field"><label>{label}</label><span>{value}</span></div>"#,
        label = html_escape(label),
        value = value
    )
}

fn not_found(entity: &str) -> Html<String> {
    Html(format!(
        r#"<div class="crm-detail-empty">No {entity} record found</div>"#,
        entity = html_escape(entity)
    ))
}

/// `/api/ui/crm/records/:entity/:id` — detail drawer fragment.
pub async fn handle_record_detail(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Path((entity, id)): Path<(String, Uuid)>,
) -> Html<String> {
    let Ok(mut conn) = state.db_pool.get() else {
        return Html(r#"<div class="crm-detail-empty">Service unavailable</div>"#.to_string());
    };
    let branch_id = branch_ctx(&state, &headers, &mut conn);

    match entity.as_str() {
        "contact" => contact_detail(&mut conn, branch_id, id),
        "account" => account_detail(&mut conn, branch_id, id),
        "deal" | "lead" | "opportunity" => deal_detail(&mut conn, branch_id, id, &entity),
        other => Html(format!(
            r#"<div class="crm-detail-empty">Unknown record type "{other}"</div>"#
        )),
    }
}

fn shell(entity: &str, id: Uuid, title: &str, subtitle: &str, fields: String, related: String,
         timeline: String) -> Html<String> {
    Html(format!(
        r#"<div class="record-detail" data-entity="{entity}" data-id="{id}">
<div class="record-detail-head"><strong>{title}</strong><span class="text-muted">{subtitle}</span><button class="record-detail-close" title="Close">×</button></div>
<div class="record-detail-body">
<div class="record-detail-fields">{fields}</div>
{related}
<div class="record-timeline"><h4 class="lead-detail-subtitle">Activity timeline</h4><form class="activity-form" data-entity="{entity}" data-id="{id}"><select name="activity_type" class="crm-form-select"><option value="note">Note</option><option value="call">Call</option><option value="email">Email</option><option value="meeting">Meeting</option><option value="task">Task</option></select><input type="text" name="subject" class="crm-form-input" placeholder="What happened?" required><button type="submit" class="btn-primary">Log</button></form>{timeline}</div>
</div>
</div>"#,
        entity = entity,
        id = id,
        title = html_escape(title),
        subtitle = html_escape(subtitle),
        fields = fields,
        related = related,
        timeline = timeline,
    ))
}

fn timeline_html(rows: Vec<CrmActivity>) -> String {
    if rows.is_empty() {
        return r#"<div class="lead-detail-empty-timeline">No activities recorded yet</div>"#
            .to_string();
    }
    let mut html = String::from(r#"<ul class="lead-timeline-list">"#);
    for a in rows {
        let when = a
            .due_date
            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
            .or_else(|| Some(a.created_at.format("%Y-%m-%d %H:%M").to_string()))
            .unwrap_or_else(|| "-".to_string());
        html.push_str(&format!(
            r#"<li class="lead-timeline-item"><span class="lead-timeline-type">{kind}</span><span class="lead-timeline-subject">{subject}</span><span class="lead-timeline-date">{when}</span></li>"#,
            kind = html_escape(&a.activity_type),
            subject = html_escape(a.subject.as_deref().unwrap_or("")),
            when = html_escape(&when),
        ));
    }
    html.push_str("</ul>");
    html
}

fn activities_for(
    conn: &mut diesel::PgConnection,
    branch_id: Uuid,
    entity: &str,
    id: Uuid,
) -> Vec<CrmActivity> {
    crm_activities::table
        .filter(crm_activities::branch_id.eq(branch_id))
        .order(crm_activities::created_at.desc())
        .limit(200)
        .load(conn)
        .unwrap_or_default()
        .into_iter()
        .filter(|a| activity_matches(a, entity, id))
        .take(20)
        .collect()
}

fn contact_detail(conn: &mut diesel::PgConnection, branch_id: Uuid, id: Uuid) -> Html<String> {
    let contact: CrmContact = match crm_contacts::table
        .filter(crm_contacts::id.eq(id))
        .filter(crm_contacts::branch_id.eq(branch_id))
        .first(conn)
    {
        Ok(c) => c,
        Err(_) => return not_found("contact"),
    };

    let deals: Vec<CrmDeal> = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .filter(crm_deals::contact_id.eq(id))
        .order(crm_deals::created_at.desc())
        .limit(20)
        .load(conn)
        .unwrap_or_default();

    let mut fields = String::new();
    fields.push_str(&field("Email", &html_escape(contact.email.as_deref().unwrap_or("-"))));
    fields.push_str(&field("Phone", &html_escape(contact.phone.as_deref().unwrap_or("-"))));
    fields.push_str(&field("Mobile", &html_escape(contact.mobile.as_deref().unwrap_or("-"))));
    fields.push_str(&field("Company", &html_escape(contact.company.as_deref().unwrap_or("-"))));
    fields.push_str(&field("Job title", &html_escape(contact.job_title.as_deref().unwrap_or("-"))));
    fields.push_str(&field("City", &html_escape(contact.city.as_deref().unwrap_or("-"))));
    fields.push_str(&field("Country", &html_escape(contact.country.as_deref().unwrap_or("-"))));
    fields.push_str(&field("Status", &html_escape(&contact.status)));
    fields.push_str(&field("Source", &html_escape(contact.source.as_deref().unwrap_or("-"))));
    fields.push_str(&field("Notes", &html_escape(contact.notes.as_deref().unwrap_or("-"))));

    let mut related = String::from(r#"<div class="record-related"><h4 class="lead-detail-subtitle">Deals</h4>"#);
    if deals.is_empty() {
        related.push_str(r#"<div class="lead-detail-empty-timeline">No deals linked yet</div>"#);
    } else {
        related.push_str(r#"<ul class="lead-timeline-list">"#);
        for d in deals {
            let label = d.title.as_deref().or(d.name.as_deref()).unwrap_or("");
            let value = d
                .value
                .map(|v| format_money(v, d.currency.as_deref()))
                .unwrap_or_else(|| "-".to_string());
            related.push_str(&format!(
                r#"<li class="lead-timeline-item"><span class="lead-timeline-type">{stage}</span><span class="lead-timeline-subject">{label}</span><span class="lead-timeline-date">{value}</span></li>"#,
                stage = html_escape(d.stage.as_deref().unwrap_or("-")),
                label = html_escape(label),
                value = html_escape(&value),
            ));
        }
        related.push_str("</ul>");
    }
    related.push_str("</div>");

    shell(
        "contact",
        contact.id,
        &full_name(&contact),
        contact.email.as_deref().unwrap_or("contact"),
        fields,
        related,
        timeline_html(activities_for(conn, branch_id, "contact", contact.id)),
    )
}

fn account_detail(conn: &mut diesel::PgConnection, branch_id: Uuid, id: Uuid) -> Html<String> {
    let account: CrmAccount = match crm_accounts::table
        .filter(crm_accounts::id.eq(id))
        .filter(crm_accounts::branch_id.eq(branch_id))
        .first(conn)
    {
        Ok(a) => a,
        Err(_) => return not_found("account"),
    };

    let deals: Vec<CrmDeal> = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .filter(crm_deals::account_id.eq(id))
        .order(crm_deals::created_at.desc())
        .limit(20)
        .load(conn)
        .unwrap_or_default();
    let contacts: Vec<CrmContact> = crm_contacts::table
        .filter(crm_contacts::branch_id.eq(branch_id))
        .filter(crm_contacts::company.eq(&account.name))
        .limit(20)
        .load(conn)
        .unwrap_or_default();

    let mut fields = String::new();
    fields.push_str(&field("Industry", &html_escape(account.industry.as_deref().unwrap_or("-"))));
    fields.push_str(&field("Website", &html_escape(account.website.as_deref().unwrap_or("-"))));
    fields.push_str(&field("Phone", &html_escape(account.phone.as_deref().unwrap_or("-"))));
    fields.push_str(&field("Email", &html_escape(account.email.as_deref().unwrap_or("-"))));
    fields.push_str(&field(
        "Employees",
        &account
            .employees_count
            .map(|c| c.to_string())
            .unwrap_or_else(|| "-".to_string()),
    ));
    fields.push_str(&field(
        "Revenue",
        &account
            .annual_revenue
            .map(|r| html_escape(&format_money(r, None)))
            .unwrap_or_else(|| "-".to_string()),
    ));
    fields.push_str(&field("City", &html_escape(account.city.as_deref().unwrap_or("-"))));
    fields.push_str(&field("Country", &html_escape(account.country.as_deref().unwrap_or("-"))));
    fields.push_str(&field("Description", &html_escape(account.description.as_deref().unwrap_or("-"))));

    let mut related = String::from(r#"<div class="record-related"><h4 class="lead-detail-subtitle">Deals &amp; contacts</h4><ul class="lead-timeline-list">"#);
    for d in &deals {
        let label = d.title.as_deref().or(d.name.as_deref()).unwrap_or("");
        let value = d
            .value
            .map(|v| format_money(v, d.currency.as_deref()))
            .unwrap_or_else(|| "-".to_string());
        related.push_str(&format!(
            r#"<li class="lead-timeline-item"><span class="lead-timeline-type">{stage}</span><span class="lead-timeline-subject">{label}</span><span class="lead-timeline-date">{value}</span></li>"#,
            stage = html_escape(d.stage.as_deref().unwrap_or("-")),
            label = html_escape(label),
            value = html_escape(&value),
        ));
    }
    for c in &contacts {
        related.push_str(&format!(
            r#"<li class="lead-timeline-item"><span class="lead-timeline-type">contact</span><span class="lead-timeline-subject">{label}</span><span class="lead-timeline-date">{email}</span></li>"#,
            label = html_escape(&full_name(c)),
            email = html_escape(c.email.as_deref().unwrap_or("")),
        ));
    }
    if deals.is_empty() && contacts.is_empty() {
        related = String::from(r#"<div class="record-related"><div class="lead-detail-empty-timeline">Nothing linked to this account yet</div></div>"#);
    } else {
        related.push_str("</ul></div>");
    }

    shell(
        "account",
        account.id,
        &account.name,
        account.industry.as_deref().unwrap_or("account"),
        fields,
        related,
        timeline_html(activities_for(conn, branch_id, "account", account.id)),
    )
}

fn deal_detail(
    conn: &mut diesel::PgConnection,
    branch_id: Uuid,
    id: Uuid,
    entity: &str,
) -> Html<String> {
    let deal: CrmDeal = match crm_deals::table
        .filter(crm_deals::id.eq(id))
        .filter(crm_deals::branch_id.eq(branch_id))
        .first(conn)
    {
        Ok(d) => d,
        Err(_) => return not_found(entity),
    };

    let contact = deal
        .contact_id
        .and_then(|cid| {
            crm_contacts::table
                .filter(crm_contacts::id.eq(cid))
                .first::<CrmContact>(conn)
                .ok()
        });
    let account = deal
        .account_id
        .and_then(|aid| {
            crm_accounts::table
                .filter(crm_accounts::id.eq(aid))
                .first::<CrmAccount>(conn)
                .ok()
        });

    let label = deal.title.clone().or(deal.name.clone()).unwrap_or_default();
    let mut fields = String::new();
    fields.push_str(&field(
        "Value",
        &deal
            .value
            .map(|v| html_escape(&format_money(v, deal.currency.as_deref())))
            .unwrap_or_else(|| "-".to_string()),
    ));
    fields.push_str(&field("Stage", &html_escape(deal.stage.as_deref().unwrap_or("-"))));
    fields.push_str(&field(
        "Probability",
        &deal
            .probability
            .map(|p| format!("{p}%"))
            .unwrap_or_else(|| "-".to_string()),
    ));
    fields.push_str(&field(
        "Contact",
        &super::lists::contact_name(contact.as_ref()),
    ));
    fields.push_str(&field(
        "Account",
        &super::lists::account_name(account.as_ref()),
    ));
    fields.push_str(&field("Source", &html_escape(deal.source.as_deref().unwrap_or("-"))));
    fields.push_str(&field(
        "Expected close",
        &deal
            .expected_close_date
            .map(|d| html_escape(&d.to_string()))
            .unwrap_or_else(|| "-".to_string()),
    ));
    fields.push_str(&field(
        "Created",
        &html_escape(&deal.created_at.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "-".to_string())),
    ));
    fields.push_str(&field("Notes", &html_escape(deal.notes.as_deref().unwrap_or("-"))));
    fields.push_str(&field(
        "Lost reason",
        &html_escape(deal.lost_reason.as_deref().unwrap_or("-")),
    ));

    let lost_input = r#"<div class="lost-reason-row"><input type="text" class="crm-form-input" id="record-lost-reason" placeholder="Lost reason (used when the stage is lost)"><button class="btn-secondary" data-action="save-lost-reason" data-entity="{entity}" data-id="{id}">Save reason</button></div>"#;

    shell(
        entity,
        deal.id,
        &label,
        deal.stage.as_deref().unwrap_or("deal"),
        fields,
        lost_input.replace("{entity}", entity).replace("{id}", &deal.id.to_string()),
        timeline_html(activities_for(conn, branch_id, entity, deal.id)),
    )
}
