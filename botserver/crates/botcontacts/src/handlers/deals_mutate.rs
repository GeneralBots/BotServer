//! #1441 — deal mutations (update/delete), split out of `crm.rs` for the
//! 450-line budget. Both snapshot the row into `crm_audit_logs` before and
//! after so the CRM audit drawer can answer who changed what, when.

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
use crate::models::{stage_probability, CrmDeal};
use crate::requests::UpdateDealRequest;
use crate::schema::crm_deals;
use crate::CrateState;

pub async fn update_deal(

    State(state): State<Arc<CrateState>>,

    headers: HeaderMap,

    Path(id): Path<Uuid>,

    Json(req): Json<UpdateDealRequest>,

) -> Result<Json<CrmDeal>, (StatusCode, String)> {

    let mut conn = state.db_pool.get().map_err(|e| {

        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))

    })?;



    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)

        .unwrap_or_else(|| state.get_bot_context());



    let before: CrmDeal = crm_deals::table

        .filter(crm_deals::id.eq(id))

        .filter(crm_deals::branch_id.eq(branch_id))

        .first(&mut conn)

        .map_err(|_| (StatusCode::NOT_FOUND, "Deal not found".to_string()))?;



    let now = Utc::now();



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



    if let Some(name) = req.name {

        diesel::update(crm_deals::table.filter(crm_deals::id.eq(id))

                .filter(crm_deals::branch_id.eq(branch_id)))

            .set(crm_deals::name.eq(name))

            .execute(&mut conn)

            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    }



    if let Some(value) = req.value {

        diesel::update(crm_deals::table.filter(crm_deals::id.eq(id))

                .filter(crm_deals::branch_id.eq(branch_id)))

            .set(crm_deals::value.eq(value))

            .execute(&mut conn)

            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    }



    if let Some(currency) = req.currency {

        diesel::update(crm_deals::table.filter(crm_deals::id.eq(id))

                .filter(crm_deals::branch_id.eq(branch_id)))

            .set(crm_deals::currency.eq(currency))

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



    if let Some(department_id) = req.department_id {

        diesel::update(crm_deals::table.filter(crm_deals::id.eq(id))

                .filter(crm_deals::branch_id.eq(branch_id)))

            .set(crm_deals::department_id.eq(department_id))

            .execute(&mut conn)

            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    }



    if let Some(owner_id) = req.owner_id {

        diesel::update(crm_deals::table.filter(crm_deals::id.eq(id))

                .filter(crm_deals::branch_id.eq(branch_id)))

            .set(crm_deals::owner_id.eq(owner_id))

            .execute(&mut conn)

            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    }



    if let Some(lost_reason) = req.lost_reason {

        diesel::update(crm_deals::table.filter(crm_deals::id.eq(id))

                .filter(crm_deals::branch_id.eq(branch_id)))

            .set(crm_deals::lost_reason.eq(lost_reason))

            .execute(&mut conn)

            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    }



    if let Some(won) = req.won {

        diesel::update(crm_deals::table.filter(crm_deals::id.eq(id))

                .filter(crm_deals::branch_id.eq(branch_id)))

            .set((crm_deals::won.eq(won), crm_deals::closed_at.eq(Some(now))))

            .execute(&mut conn)

            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    }



    if let Some(notes) = req.notes {

        diesel::update(crm_deals::table.filter(crm_deals::id.eq(id))

                .filter(crm_deals::branch_id.eq(branch_id)))

            .set(crm_deals::notes.eq(notes))

            .execute(&mut conn)

            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    }



    if let Some(tags) = req.tags {

        diesel::update(crm_deals::table.filter(crm_deals::id.eq(id))

                .filter(crm_deals::branch_id.eq(branch_id)))

            .set(crm_deals::tags.eq(Some(tags)))

            .execute(&mut conn)

            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    }



    let after: CrmDeal = crm_deals::table

        .filter(crm_deals::id.eq(id))

        .filter(crm_deals::branch_id.eq(branch_id))

        .first(&mut conn)

        .unwrap_or_else(|_| before.clone());

    audit::record(
        &state,
        audit::AuditRecord {
            headers: &headers,
            branch_id,
            entity: "deal",
            entity_id: Some(id),
            action: "update",
            before: Some(serde_json::to_value(&before).unwrap_or(serde_json::Value::Null)),
            after: Some(serde_json::to_value(&after).unwrap_or(serde_json::Value::Null)),
            detail: None,
        },
    );

    Ok(Json(after))

}

pub async fn delete_deal(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| state.get_bot_context());

    let before: Option<CrmDeal> = crm_deals::table
        .filter(crm_deals::id.eq(id))
        .filter(crm_deals::branch_id.eq(branch_id))
        .first(&mut conn)
        .ok();

    // #1441 C5 — destructive action restricted to the record owner or an admin.
    crate::authz::ensure_can_modify(&mut conn, &headers, before.as_ref().and_then(|d| d.owner_id))?;

    diesel::delete(crm_deals::table.filter(crm_deals::id.eq(id))
                .filter(crm_deals::branch_id.eq(branch_id)))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Delete error: {e}")))?;

    if let Some(row) = before {
        audit::record(
            &state,
            audit::AuditRecord {
                headers: &headers,
                branch_id,
                entity: "deal",
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
