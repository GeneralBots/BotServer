use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use axum::response::IntoResponse;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::audit;
use crate::models::CrmActivity;
use crate::requests::{CreateActivityRequest, ListQuery};
use crate::schema::crm_activities;
use crate::scope::branch_from_jwt;
use crate::CrateState;

fn get_bot_context(state: &CrateState) -> Uuid {
    state.get_bot_context()
}

pub async fn list_activities(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Result<axum::response::Response, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    // #1441 P2 — X-Total-Count lets the UI paginate without a second probe.
    let total: i64 = crm_activities::table
        .filter(crm_activities::branch_id.eq(branch_id))
        .count()
        .get_result(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Count error: {e}")))?;

    let activities: Vec<CrmActivity> = crm_activities::table
        .filter(crm_activities::branch_id.eq(branch_id))
        .order(crm_activities::created_at.desc())
        .limit(limit)
        .offset(offset)
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(([("X-Total-Count", total.to_string())], Json(activities)).into_response())
}

pub async fn create_activity(
    State(state): State<Arc<CrateState>>,
    headers: HeaderMap,
    Json(req): Json<CreateActivityRequest>,
) -> Result<Json<CrmActivity>, (StatusCode, String)> {
    let mut conn = state.db_pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| state.get_bot_context());
    let id = Uuid::new_v4();
    let now = Utc::now();

    let due_date = req.due_date
        .and_then(|d| DateTime::parse_from_rfc3339(&d).ok())
        .map(|d| d.with_timezone(&Utc));

    let activity = CrmActivity {
        id,
        org_id: state.org_for_branch(branch_id),
        bot_id: state.bot_for_branch(branch_id),
        branch_id,
        contact_id: req.contact_id,
        activity_type: req.activity_type,
        subject: req.subject,
        description: req.description,
        due_date,
        completed_at: None,
        created_at: now,
        lead_id: req.lead_id,
        opportunity_id: req.opportunity_id,
        account_id: req.account_id,
        outcome: None,
        owner_id: None,
    };

    diesel::insert_into(crm_activities::table)
        .values(&activity)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    audit::record(
        &state,
        audit::AuditRecord {
            headers: &headers,
            branch_id,
            entity: "activity",
            entity_id: Some(id),
            action: "create",
            before: None,
            after: Some(serde_json::json!({
            "activity_type": activity.activity_type,
            "subject": activity.subject,
        })),
            detail: None,
        },
    );
    Ok(Json(activity))
}
