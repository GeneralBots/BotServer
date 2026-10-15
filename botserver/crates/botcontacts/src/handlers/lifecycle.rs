//! #1441 — lead lifecycle mutations (update + delete + convert), split out of
//! `deals.rs` to keep every file under the 450-line budget (#1443 policy).
//! The state-changing paths write audit rows; readers stay in `deals.rs`.

use axum::{
    extract::{Path, State},
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
use crate::schema::crm_deals;
use crate::CrateState;

fn get_bot_context(state: &CrateState) -> Uuid {
    state.get_bot_context()
}

/// `DELETE /api/crm/leads/:id` — removes the lead after snapshotting it into
/// the audit trail (branch-scoped; foreign-branch ids match nothing).
pub async fn delete_lead(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| get_bot_context(&state));

    // Snapshot before the delete so the audit row carries the full record.
    let before: Option<CrmDeal> = crm_deals::table
        .filter(crm_deals::id.eq(id))
        .filter(crm_deals::branch_id.eq(branch_id))
        .first(&mut conn)
        .ok();

    // #1441 A3 — a converted lead still backs a live opportunity: deleting it
    // would orphan the pipeline row, so the delete is refused with the count
    // instead of failing later on the foreign key.
    let converted: i64 = crm_deals::table
        .filter(crm_deals::branch_id.eq(branch_id))
        .filter(crm_deals::lead_id.eq(id))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);
    if converted > 0 {
        return Err((
            StatusCode::CONFLICT,
            format!("lead backs {converted} converted opportunity/ies; delete or reassign them first"),
        ));
    }

    // #1441 C5 — destructive action restricted to the record owner or an admin.
    crate::authz::ensure_can_modify(
        &mut conn,
        &headers,
        before.as_ref().and_then(|d| d.owner_id),
    )?;

    diesel::delete(
        crm_deals::table
            .filter(crm_deals::id.eq(id))
            .filter(crm_deals::branch_id.eq(branch_id)),
    )
    .execute(&mut conn)
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Delete error: {e}")))?;

    if let Some(row) = before {
        audit::record(
            &state,
            audit::AuditRecord {
                headers: &headers,
                branch_id,
                entity: "lead",
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

/// `POST /api/crm/leads/:id/convert` — turns a lead into a qualification-stage
/// opportunity, closing the lead as `converted`. Audited as `convert` with the
/// created opportunity as the `after` snapshot.
pub async fn convert_lead_to_opportunity(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<CrmDeal>, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| get_bot_context(&state));

    let lead: CrmDeal = crm_deals::table
        .filter(crm_deals::id.eq(id))
        .filter(crm_deals::branch_id.eq(branch_id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Lead not found".to_string()))?;

    let opp_id = Uuid::new_v4();
    let now = Utc::now();

    let opportunity = CrmDeal {
        id: opp_id,
        org_id: lead.org_id,
        bot_id: lead.bot_id,
        branch_id: lead.branch_id,
        lead_id: Some(lead.id),
        account_id: lead.account_id,
        contact_id: lead.contact_id,
        am_id: None,
        title: lead.title.clone(),
        name: lead.title.clone().unwrap_or_default(),
        description: lead.description.clone(),
        value: lead.value,
        currency: lead.currency.clone(),
        stage_id: None,
        stage: Some("qualification".to_string()),
        probability: Some(25),
        source: lead.source.clone(),
        segment_id: None,
        department_id: None,
        expected_close_date: lead.expected_close_date,
        actual_close_date: None,
        period: None,
        deal_date: None,
        won: None,
        owner_id: lead.owner_id,
        lost_reason: None,
        closed_at: None,
        notes: None,
        tags: lead.tags.clone(),
        custom_fields: lead.custom_fields.clone(),
        created_at: now,
        updated_at: now,
    };

    diesel::insert_into(crm_deals::table)
        .values(&opportunity)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    diesel::update(
        crm_deals::table
            .filter(crm_deals::id.eq(id))
            .filter(crm_deals::branch_id.eq(branch_id)),
    )
    .set((crm_deals::stage.eq("converted"), crm_deals::closed_at.eq(Some(now))))
    .execute(&mut conn)
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    audit::record(
        &state,
        audit::AuditRecord {
            headers: &headers,
            branch_id,
            entity: "lead",
            entity_id: Some(id),
            action: "convert",
            before: Some(serde_json::to_value(&lead).unwrap_or(serde_json::Value::Null)),
            after: Some(serde_json::to_value(&opportunity).unwrap_or(serde_json::Value::Null)),
            detail: Some(serde_json::json!({ "opportunity_id": opp_id })),
        },
    );

    Ok(Json(opportunity))
}

pub async fn update_lead(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateLeadRequest>,
) -> Result<Json<CrmDeal>, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| get_bot_context(&state));

    let now = Utc::now();

    // #1441 P2 — audit trail records which fields the caller sent (captured
    // before the fields are moved out below).
    let changed_fields: Vec<&str> = [
        (req.title.is_some(), "title"),
        (req.stage.is_some(), "stage"),
        (req.lost_reason.is_some(), "lost_reason"),
    ]
    .iter()
    .filter(|(changed, _)| *changed)
    .map(|(_, field)| *field)
    .collect();

    diesel::update(crm_deals::table.filter(crm_deals::id.eq(id))
                .filter(crm_deals::branch_id.eq(branch_id)))
        .set(crm_deals::updated_at.eq(now))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    if let Some(title) = req.title {
        diesel::update(crm_deals::table.filter(crm_deals::id.eq(id))
                .filter(crm_deals::branch_id.eq(branch_id)))
            .set(crm_deals::title.eq(title))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    if let Some(stage) = req.stage {
        let probability = stage_probability(&stage);
        diesel::update(crm_deals::table.filter(crm_deals::id.eq(id))
                .filter(crm_deals::branch_id.eq(branch_id)))
            .set((crm_deals::stage.eq(&stage), crm_deals::probability.eq(probability)))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

        if stage == "won" || stage == "lost" {
            diesel::update(crm_deals::table.filter(crm_deals::id.eq(id))
                .filter(crm_deals::branch_id.eq(branch_id)))
                .set(crm_deals::closed_at.eq(Some(now)))
                .execute(&mut conn)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
        }
    }

    if let Some(lost_reason) = req.lost_reason {
        diesel::update(crm_deals::table.filter(crm_deals::id.eq(id))
                .filter(crm_deals::branch_id.eq(branch_id)))
            .set(crm_deals::lost_reason.eq(lost_reason))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    // Field-wise patches do not carry a meaningful before/after snapshot; the
    // audit row records which fields the caller sent.
    audit::record(
        &state,
        audit::AuditRecord {
            headers: &headers,
            branch_id,
            entity: "lead",
            entity_id: Some(id),
            action: "update",
            before: None,
            after: None,
            detail: Some(serde_json::json!({ "fields": changed_fields })),
        },
    );

    super::deals::get_lead(State(state), headers, Path(id)).await
}
