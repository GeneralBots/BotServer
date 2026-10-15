//! #1441 P2 — bulk actions, CSV import/export and the audit-trail reader.
//!
//! All handlers resolve the caller's branch from the JWT/session token and
//! never touch rows outside it; every mutating path writes an audit row via
//! `crate::audit::record`.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::audit;
use crate::models::{CrmContact, CrmDeal};
use crate::requests::{LeadBulkActionRequest, LeadBulkActionResult};
use crate::schema::{crm_contacts, crm_deals};
use crate::scope::branch_from_jwt;
use crate::CrateState;

fn get_bot_context(state: &CrateState) -> Uuid {
    state.get_bot_context()
}

fn db_err<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
}

// ────────────────────────────────────────────────────────────────────────────
// Bulk actions
// ────────────────────────────────────────────────────────────────────────────

/// `POST /api/crm/leads/bulk` — stage/owner/delete over a set of lead ids.
/// Ids that match no row inside the branch are reported in `failed`.
pub async fn bulk_leads(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Json(req): Json<LeadBulkActionRequest>,
) -> Result<Json<LeadBulkActionResult>, (StatusCode, String)> {
    if req.ids.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "ids must not be empty".to_string()));
    }
    if req.ids.len() > 500 {
        return Err((StatusCode::BAD_REQUEST, "at most 500 ids per request".to_string()));
    }
    let action = req.action.as_str();
    if !matches!(action, "stage" | "owner" | "delete") {
        return Err((StatusCode::BAD_REQUEST, format!("unsupported action '{action}'")));
    }
    if action == "stage" && req.stage.is_none() {
        return Err((StatusCode::BAD_REQUEST, "action 'stage' requires a stage".to_string()));
    }
    if action == "owner" && req.owner_id.is_none() {
        return Err((StatusCode::BAD_REQUEST, "action 'owner' requires owner_id".to_string()));
    }

    let mut conn = state.db_pool.get().map_err(db_err)?;
    let branch_id = branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));

    // Snapshot before-values for the audit trail in one round trip.
    let before_rows: Vec<CrmDeal> = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .filter(crm_deals::id.eq_any(&req.ids))
        .load(&mut conn)
        .map_err(db_err)?;
    let failed: Vec<Uuid> = req
        .ids
        .iter()
        .filter(|id| !before_rows.iter().any(|r| &r.id == *id))
        .copied()
        .collect();

    let result = match action {
        "stage" => {
            let stage = req.stage.clone().unwrap_or_default();
            let updated = diesel::update(
                crm_deals::table
                    .filter(crm_deals::branch_id.eq(branch_id))
                    .filter(crm_deals::id.eq_any(&req.ids)),
            )
            .set((
                crm_deals::stage.eq(stage.clone()),
                crm_deals::updated_at.eq(chrono::Utc::now()),
            ))
            .execute(&mut conn)
            .map_err(db_err)? as u64;
            for row in &before_rows {
                audit::record(
                    &state,
                    audit::AuditRecord {
                        headers: &headers,
                        branch_id,
                        entity: "lead",
                        entity_id: Some(row.id),
                        action: "bulk_stage",
                        before: Some(serde_json::json!({ "stage": row.stage })),
                        after: Some(serde_json::json!({ "stage": stage })),
                        detail: None,
                    },
                );
            }
            LeadBulkActionResult { updated, deleted: 0, failed }
        }
        "owner" => {
            let owner = req.owner_id.unwrap_or_default();
            let updated = diesel::update(
                crm_deals::table
                    .filter(crm_deals::branch_id.eq(branch_id))
                    .filter(crm_deals::id.eq_any(&req.ids)),
            )
            .set((
                crm_deals::owner_id.eq(Some(owner)),
                crm_deals::updated_at.eq(chrono::Utc::now()),
            ))
            .execute(&mut conn)
            .map_err(db_err)? as u64;
            for row in &before_rows {
                audit::record(
                    &state,
                    audit::AuditRecord {
                        headers: &headers,
                        branch_id,
                        entity: "lead",
                        entity_id: Some(row.id),
                        action: "bulk_owner",
                        before: Some(serde_json::json!({ "owner_id": row.owner_id })),
                        after: Some(serde_json::json!({ "owner_id": owner })),
                        detail: None,
                    },
                );
            }
            LeadBulkActionResult { updated, deleted: 0, failed }
        }
        _ => {
            // #1441 C5 — bulk delete honours the same owner/admin rule.
            for row in &before_rows {
                crate::authz::ensure_can_modify(&mut conn, &headers, row.owner_id)?;
            }
            let deleted = diesel::delete(
                crm_deals::table
                    .filter(crm_deals::branch_id.eq(branch_id))
                    .filter(crm_deals::id.eq_any(&req.ids)),
            )
            .execute(&mut conn)
            .map_err(db_err)? as u64;
            for row in &before_rows {
                audit::record(
                    &state,
                    audit::AuditRecord {
                        headers: &headers,
                        branch_id,
                        entity: "lead",
                        entity_id: Some(row.id),
                        action: "bulk_delete",
                        before: Some(serde_json::to_value(row).unwrap_or(serde_json::Value::Null)),
                        after: None,
                        detail: None,
                    },
                );
            }
            LeadBulkActionResult { updated: 0, deleted, failed }
        }
    };

    Ok(Json(result))
}

/// `POST /api/crm/contacts/bulk` — owner/delete over a set of contact ids.
pub async fn bulk_contacts(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Json(req): Json<LeadBulkActionRequest>,
) -> Result<Json<LeadBulkActionResult>, (StatusCode, String)> {
    if req.ids.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "ids must not be empty".to_string()));
    }
    if req.ids.len() > 500 {
        return Err((StatusCode::BAD_REQUEST, "at most 500 ids per request".to_string()));
    }
    if !matches!(req.action.as_str(), "owner" | "delete") {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("unsupported action '{}' for contacts", req.action),
        ));
    }

    let mut conn = state.db_pool.get().map_err(db_err)?;
    let branch_id = branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));

    let before_rows: Vec<CrmContact> = crm_contacts::table
        .filter(crm_contacts::branch_id.eq(branch_id))
        .filter(crm_contacts::id.eq_any(&req.ids))
        .load(&mut conn)
        .map_err(db_err)?;
    let failed: Vec<Uuid> = req
        .ids
        .iter()
        .filter(|id| !before_rows.iter().any(|r| &r.id == *id))
        .copied()
        .collect();

    let result = match req.action.as_str() {
        "owner" => {
            let owner = req.owner_id.unwrap_or_default();
            let updated = diesel::update(
                crm_contacts::table
                    .filter(crm_contacts::branch_id.eq(branch_id))
                    .filter(crm_contacts::id.eq_any(&req.ids)),
            )
            .set((
                crm_contacts::owner_id.eq(Some(owner)),
                crm_contacts::updated_at.eq(chrono::Utc::now()),
            ))
            .execute(&mut conn)
            .map_err(db_err)? as u64;
            for row in &before_rows {
                audit::record(
                    &state,
                    audit::AuditRecord {
                        headers: &headers,
                        branch_id,
                        entity: "contact",
                        entity_id: Some(row.id),
                        action: "bulk_owner",
                        before: Some(serde_json::json!({ "owner_id": row.owner_id })),
                        after: Some(serde_json::json!({ "owner_id": owner })),
                        detail: None,
                    },
                );
            }
            LeadBulkActionResult { updated, deleted: 0, failed }
        }
        _ => {
            // #1441 C5 — bulk delete honours the same owner/admin rule.
            for row in &before_rows {
                crate::authz::ensure_can_modify(&mut conn, &headers, row.owner_id)?;
            }
            let deleted = diesel::delete(
                crm_contacts::table
                    .filter(crm_contacts::branch_id.eq(branch_id))
                    .filter(crm_contacts::id.eq_any(&req.ids)),
            )
            .execute(&mut conn)
            .map_err(db_err)? as u64;
            for row in &before_rows {
                audit::record(
                    &state,
                    audit::AuditRecord {
                        headers: &headers,
                        branch_id,
                        entity: "contact",
                        entity_id: Some(row.id),
                        action: "bulk_delete",
                        before: Some(serde_json::to_value(row).unwrap_or(serde_json::Value::Null)),
                        after: None,
                        detail: None,
                    },
                );
            }
            LeadBulkActionResult { updated: 0, deleted, failed }
        }
    };

    Ok(Json(result))
}
