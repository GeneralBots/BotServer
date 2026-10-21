//! #1441 — accounts CRUD. `PUT /api/crm/accounts/:id` was missing entirely
//! (the suite could only create, read and delete), so account edits were
//! impossible from the CRM app; every mutation here is now audited (#1441 P2)
//! and creation dedupes by name inside the branch (A2).

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
use crate::schema::{crm_accounts, crm_deals};
use crate::scope::branch_from_jwt;
use crate::CrateState;

fn get_bot_context(state: &CrateState) -> Uuid {
    state.get_bot_context()
}

fn db_err<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
}

/// A single record lookup answers 404 only when the row genuinely does not
/// exist inside the caller's branch. Mapping every diesel error to 404 used to
/// hide row-decoding failures (e.g. a `NUMERIC` money column against an `f64`
/// model) behind a misleading "not found".
fn not_found_or_500(result: Result<CrmAccount, diesel::result::Error>) -> Result<CrmAccount, (StatusCode, String)> {
    use diesel::result::Error as DieselError;
    match result {
        Ok(row) => Ok(row),
        Err(DieselError::NotFound) => Err((StatusCode::NOT_FOUND, "Account not found".to_string())),
        Err(e) => {
            log::error!("[crm] account lookup failed: {e}");
            Err(db_err(e))
        }
    }
}

/// `POST /api/crm/accounts` — find-or-create by name so the same company is
/// never duplicated by two captures (A2).
pub async fn create_account(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Json(req): Json<CreateAccountRequest>,
) -> Result<Json<CrmAccount>, (StatusCode, String)> {
    let name = req.name.trim();
    if name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "name is required".to_string()));
    }
    let mut conn = state.db_pool.get().map_err(db_err)?;
    let branch_id = branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));

    if let Some(existing) = find_by_name(&mut conn, branch_id, name).await {
        audit::record(
            &state,
            audit::AuditRecord {
                headers: &headers,
                branch_id,
                entity: "account",
                entity_id: Some(existing.id),
                action: "create_deduplicated",
                before: None,
                after: Some(serde_json::json!({ "name": existing.name })),
                detail: None,
            },
        );
        return Ok(Json(existing));
    }

    let id = Uuid::new_v4();
    let now = Utc::now();
    let account = CrmAccount {
        id,
        org_id: branch_id,
        bot_id: state.bot_for_branch(branch_id),
        branch_id,
        name: name.to_string(),
        industry: req.industry,
        website: req.website,
        phone: req.phone,
        email: req.email,
        owner_id: None,
        created_at: now,
        updated_at: now,
        employees_count: req.employees_count,
        annual_revenue: req.annual_revenue,
        address_line1: req.address_line1,
        address_line2: req.address_line2,
        city: req.city,
        state: req.state,
        postal_code: req.postal_code,
        country: req.country,
        description: req.description,
        tags: req.tags.map(|v| v.into_iter().map(Some).collect()),
        custom_fields: Some(serde_json::json!({})),
    };

    diesel::insert_into(crm_accounts::table)
        .values(&account)
        .execute(&mut conn)
        .map_err(db_err)?;

    audit::record(
        &state,
        audit::AuditRecord {
            headers: &headers,
            branch_id,
            entity: "account",
            entity_id: Some(id),
            action: "create",
            before: None,
            after: Some(serde_json::to_value(&account).unwrap_or(serde_json::Value::Null)),
            detail: None,
        },
    );
    Ok(Json(account))
}

async fn find_by_name(
    conn: &mut diesel::PgConnection,
    branch_id: Uuid,
    name: &str,
) -> Option<CrmAccount> {
    crm_accounts::table
        .filter(crm_accounts::branch_id.eq(branch_id))
        .filter(crm_accounts::name.ilike(name))
        .first::<CrmAccount>(conn)
        .ok()
}

pub async fn list_accounts(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Result<axum::response::Response, (StatusCode, String)> {
    use axum::response::IntoResponse;
    let mut conn = state.db_pool.get().map_err(db_err)?;
    let branch_id = branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    // Built twice (count + page) because boxed diesel queries are consumed
    // on execution; the closure keeps the filter list in one place (#1441 P2).
    let make_q = || {
        let mut q = crm_accounts::table
            .filter(crm_accounts::branch_id.eq(branch_id))
            .into_boxed();
        if let Some(search) = &query.search {
            let pattern = format!("%{search}%");
            q = q.filter(
                crm_accounts::name
                    .ilike(pattern.clone())
                    .or(crm_accounts::industry.ilike(pattern)),
            );
        }
        q
    };

    // #1441 P2 — X-Total-Count lets the UI paginate without a second probe.
    let total: i64 = make_q().count().get_result(&mut conn).map_err(db_err)?;

    let accounts: Vec<CrmAccount> = make_q()
        .order(crm_accounts::created_at.desc())
        .limit(limit)
        .offset(offset)
        .load(&mut conn)
        .map_err(db_err)?;

    Ok(([("X-Total-Count", total.to_string())], Json(accounts)).into_response())
}

pub async fn get_account(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<CrmAccount>, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(db_err)?;
    let branch_id = branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));

    let account = not_found_or_500(
        crm_accounts::table
            .filter(crm_accounts::id.eq(id))
            .filter(crm_accounts::branch_id.eq(branch_id))
            .first(&mut conn),
    )?;

    Ok(Json(account))
}

/// `PUT /api/crm/accounts/:id` — partial update; only the fields present in
/// the request are written, and the whole row is snapshotted before/after.
pub async fn update_account(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateAccountRequest>,
) -> Result<Json<CrmAccount>, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(db_err)?;
    let branch_id = branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));

    let before = not_found_or_500(
        crm_accounts::table
            .filter(crm_accounts::id.eq(id))
            .filter(crm_accounts::branch_id.eq(branch_id))
            .first(&mut conn),
    )?;

    diesel::update(
        crm_accounts::table
            .filter(crm_accounts::id.eq(id))
            .filter(crm_accounts::branch_id.eq(branch_id)),
    )
    .set((
        req.name
            .as_deref()
            .map(|n| crm_accounts::name.eq(n.trim().to_string())),
        req.industry.clone().map(|v| crm_accounts::industry.eq(v)),
        req.website.clone().map(|v| crm_accounts::website.eq(v)),
        req.phone.clone().map(|v| crm_accounts::phone.eq(v)),
        req.email.clone().map(|v| crm_accounts::email.eq(v)),
        req.employees_count.map(|v| crm_accounts::employees_count.eq(v)),
        req.annual_revenue.map(|v| crm_accounts::annual_revenue.eq(v)),
        req.city.clone().map(|v| crm_accounts::city.eq(v)),
        req.state.clone().map(|v| crm_accounts::state.eq(v)),
        req.postal_code.clone().map(|v| crm_accounts::postal_code.eq(v)),
        req.country.clone().map(|v| crm_accounts::country.eq(v)),
        req.description.clone().map(|v| crm_accounts::description.eq(v)),
        req.owner_id.map(|v| crm_accounts::owner_id.eq(v)),
        req.tags.clone().map(|v| crm_accounts::tags.eq(v)),
        crm_accounts::updated_at.eq(Utc::now()),
    ))
    .execute(&mut conn)
    .map_err(db_err)?;

    let after = not_found_or_500(
        crm_accounts::table
            .filter(crm_accounts::id.eq(id))
            .filter(crm_accounts::branch_id.eq(branch_id))
            .first(&mut conn),
    )?;

    audit::record(
        &state,
        audit::AuditRecord {
            headers: &headers,
            branch_id,
            entity: "account",
            entity_id: Some(id),
            action: "update",
            before: Some(serde_json::to_value(&before).unwrap_or(serde_json::Value::Null)),
            after: Some(serde_json::to_value(&after).unwrap_or(serde_json::Value::Null)),
            detail: None,
        },
    );
    Ok(Json(after))
}

pub async fn delete_account(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(db_err)?;
    let branch_id = branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));

    let before: Option<CrmAccount> = crm_accounts::table
        .filter(crm_accounts::id.eq(id))
        .filter(crm_accounts::branch_id.eq(branch_id))
        .first(&mut conn)
        .ok();

    // #1441 — an account still referenced by deals is refused with the count
    // instead of failing later on the `crm_deals.account_id` foreign key (the
    // raw error surfaced as a 500 with no explanation).
    let linked_deals: i64 = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .filter(crm_deals::account_id.eq(id))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);
    if linked_deals > 0 {
        return Err((
            StatusCode::CONFLICT,
            format!("account backs {linked_deals} deal(s); delete or reassign them first"),
        ));
    }

    // #1441 C5 — destructive action restricted to the record owner or an admin.
    crate::authz::ensure_can_modify(&mut conn, &headers, before.as_ref().and_then(|a| a.owner_id))?;

    diesel::delete(
        crm_accounts::table
            .filter(crm_accounts::id.eq(id))
            .filter(crm_accounts::branch_id.eq(branch_id)),
    )
    .execute(&mut conn)
    .map_err(db_err)?;

    if let Some(row) = before {
        audit::record(
            &state,
            audit::AuditRecord {
                headers: &headers,
                branch_id,
                entity: "account",
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
