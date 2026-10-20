use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::Utc;
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::audit;
use crate::models::*;
use crate::requests::*;
use crate::schema::crm_contacts;
use crate::scope::branch_from_jwt;
use crate::CrateState;

fn get_bot_context(state: &CrateState) -> Uuid {
    state.get_bot_context()
}

fn db_err<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
}

pub async fn create_contact(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Json(req): Json<CreateContactRequest>,
) -> Result<Json<CrmContact>, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| get_bot_context(&state));

    // #1441 A2 — email is the dedupe key inside the branch: creating a second
    // row for the same person silently split their pipeline in two.
    let email = req
        .email
        .as_deref()
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .map(|e| e.to_lowercase());
    if let Some(mail) = &email {
        let existing = find_by_email(&mut conn, branch_id, mail).await;
        if let Some(found) = existing {
            audit::record(
                &state,
                audit::AuditRecord {
                    headers: &headers,
                    branch_id,
                    entity: "contact",
                    entity_id: Some(found.id),
                    action: "create_deduplicated",
                    before: None,
                    after: Some(serde_json::json!({ "email": found.email })),
                    detail: None,
                },
            );
            return Ok(Json(found));
        }
    }

    let id = Uuid::new_v4();
    let now = Utc::now();

    let contact = CrmContact {
        id,
        org_id: branch_id,
        bot_id: state.bot_for_branch(branch_id),
        branch_id,
        first_name: req.first_name,
        last_name: req.last_name,
        email: req.email,
        phone: req.phone,
        mobile: req.mobile,
        company: req.company,
        job_title: req.job_title,
        source: req.source,
        status: "active".to_string(),
        tags: req.tags.map(|v| v.into_iter().map(Some).collect()),
        custom_fields: Some(serde_json::json!({})),
        notes: req.notes,
        owner_id: None,
        pass_hash: None,
        created_at: now,
        updated_at: now,
        address_line1: req.address_line1,
        address_line2: None,
        city: req.city,
        state: req.state,
        postal_code: req.postal_code,
        country: req.country,
    };

    diesel::insert_into(crm_contacts::table)
        .values(&contact)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    (state.trigger_contact_change)(&mut conn, id, "created", branch_id);

    audit::record(
        &state,
        audit::AuditRecord {
            headers: &headers,
            branch_id,
            entity: "contact",
            entity_id: Some(id),
            action: "create",
            before: None,
            after: Some(serde_json::to_value(&contact).unwrap_or(serde_json::Value::Null)),
            detail: None,
        },
    );

    Ok(Json(contact))
}

async fn find_by_email(
    conn: &mut diesel::PgConnection,
    branch_id: Uuid,
    email: &str,
) -> Option<CrmContact> {
    crm_contacts::table
        .filter(crm_contacts::branch_id.eq(branch_id))
        .filter(crm_contacts::email.ilike(email))
        .first::<CrmContact>(conn)
        .ok()
}

pub async fn list_contacts(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Result<axum::response::Response, (StatusCode, String)> {
    use axum::response::IntoResponse;
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| get_bot_context(&state));
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    // Built twice (count + page) because boxed diesel queries are consumed
    // on execution; the closure keeps the filter list in one place (#1441 P2).
    let make_q = || {
        let mut q = crm_contacts::table
            .filter(crm_contacts::branch_id.eq(branch_id))
            .into_boxed();
        if let Some(status) = &query.status {
            q = q.filter(crm_contacts::status.eq(status.clone()));
        }
        if let Some(search) = &query.search {
            let pattern = format!("%{search}%");
            q = q.filter(
                crm_contacts::first_name.ilike(pattern.clone())
                    .or(crm_contacts::last_name.ilike(pattern.clone()))
                    .or(crm_contacts::email.ilike(pattern.clone()))
                    .or(crm_contacts::company.ilike(pattern)),
            );
        }
        q
    };

    // #1441 P2 — X-Total-Count lets the UI paginate without a second probe.
    let total: i64 = make_q()
        .count()
        .get_result(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Count error: {e}")))?;

    let contacts: Vec<CrmContact> = make_q()
        .order(crm_contacts::created_at.desc())
        .limit(limit)
        .offset(offset)
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(([("X-Total-Count", total.to_string())], Json(contacts)).into_response())
}

pub async fn get_contact(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<CrmContact>, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| get_bot_context(&state));

    let contact: CrmContact = crm_contacts::table
        .filter(crm_contacts::id.eq(id))
                .filter(crm_contacts::branch_id.eq(branch_id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Contact not found".to_string()))?;

    Ok(Json(contact))
}

/// `PUT /api/crm/contacts/:id` — one statement, one audit row. The previous
/// field-by-field update ran a query per field and wrote no audit trail, so
/// "who changed this contact" was unanswerable (C5).
pub async fn update_contact(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateContactRequest>,
) -> Result<Json<CrmContact>, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(db_err)?;
    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| get_bot_context(&state));

    let before: CrmContact = crm_contacts::table
        .filter(crm_contacts::id.eq(id))
        .filter(crm_contacts::branch_id.eq(branch_id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Contact not found".to_string()))?;

    diesel::update(
        crm_contacts::table
            .filter(crm_contacts::id.eq(id))
            .filter(crm_contacts::branch_id.eq(branch_id)),
    )
    .set((
        req.first_name.clone().map(|v| crm_contacts::first_name.eq(v)),
        req.last_name.clone().map(|v| crm_contacts::last_name.eq(v)),
        req.email.clone().map(|v| crm_contacts::email.eq(v)),
        req.phone.clone().map(|v| crm_contacts::phone.eq(v)),
        req.mobile.clone().map(|v| crm_contacts::mobile.eq(v)),
        req.company.clone().map(|v| crm_contacts::company.eq(v)),
        req.job_title.clone().map(|v| crm_contacts::job_title.eq(v)),
        req.status.clone().map(|v| crm_contacts::status.eq(v)),
        req.tags.clone().map(|v| crm_contacts::tags.eq(v)),
        req.notes.clone().map(|v| crm_contacts::notes.eq(v)),
        req.owner_id.map(|v| crm_contacts::owner_id.eq(v)),
        crm_contacts::updated_at.eq(Utc::now()),
    ))
    .execute(&mut conn)
    .map_err(db_err)?;

    (state.trigger_contact_change)(&mut conn, id, "updated", branch_id);

    let after: CrmContact = crm_contacts::table
        .filter(crm_contacts::id.eq(id))
        .filter(crm_contacts::branch_id.eq(branch_id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Contact not found".to_string()))?;

    audit::record(
        &state,
        audit::AuditRecord {
            headers: &headers,
            branch_id,
            entity: "contact",
            entity_id: Some(id),
            action: "update",
            before: Some(serde_json::to_value(&before).unwrap_or(serde_json::Value::Null)),
            after: Some(serde_json::to_value(&after).unwrap_or(serde_json::Value::Null)),
            detail: None,
        },
    );

    Ok(Json(after))
}

/// `DELETE /api/crm/contacts/:id` — snapshots the row into the audit trail
/// before removing it.
pub async fn delete_contact(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(db_err)?;
    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| get_bot_context(&state));

    let before: Option<CrmContact> = crm_contacts::table
        .filter(crm_contacts::id.eq(id))
        .filter(crm_contacts::branch_id.eq(branch_id))
        .first(&mut conn)
        .ok();

    // #1441 C5 — destructive action restricted to the record owner or an admin.
    crate::authz::ensure_can_modify(&mut conn, &headers, before.as_ref().and_then(|c| c.owner_id))?;

    diesel::delete(
        crm_contacts::table
            .filter(crm_contacts::id.eq(id))
            .filter(crm_contacts::branch_id.eq(branch_id)),
    )
    .execute(&mut conn)
    .map_err(db_err)?;

    (state.trigger_contact_change)(&mut conn, id, "deleted", branch_id);

    if let Some(row) = before {
        audit::record(
            &state,
            audit::AuditRecord {
                headers: &headers,
                branch_id,
                entity: "contact",
                entity_id: Some(id),
                action: "delete",
                before: Some(serde_json::to_value(&row).unwrap_or(serde_json::Value::Null)),
                after: None,
                detail: None,
            },
        );
    }
    Ok(StatusCode::NO_CONTENT)
}
