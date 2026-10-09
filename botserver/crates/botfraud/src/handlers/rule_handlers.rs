//! Rules, events and blocklist endpoints.

use crate::types::*;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use super::rows::{BlocklistRow, EventRow, RuleRow};
use super::state::{resolve_branch, FraudState};

pub async fn list_rules(
    State(state): State<Arc<FraudState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<FraudRule>>, StatusCode> {
    let branch = resolve_branch(&headers);
    let mut conn = state.pool.get().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let rules = diesel::sql_query(
        "SELECT id, branch_id, name, description, rule_type, condition_json, \
         action, severity, is_active, created_at \
         FROM fraud_rules WHERE branch_id = $1 ORDER BY severity DESC, name",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load::<RuleRow>(&mut conn)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .into_iter()
    .map(|r| FraudRule {
        id: r.id,
        branch_id: r.branch_id,
        name: r.name,
        description: r.description,
        rule_type: r.rule_type,
        condition_json: r.condition_json,
        action: r.action,
        severity: r.severity,
        is_active: r.is_active,
        created_at: r.created_at,
    })
    .collect();
    Ok(Json(rules))
}

pub async fn create_rule(
    State(state): State<Arc<FraudState>>,
    headers: HeaderMap,
    Json(payload): Json<CreateRuleRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let branch = resolve_branch(&headers);
    let mut conn = state.pool.get().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let id = Uuid::new_v4();

    diesel::sql_query(
        "INSERT INTO fraud_rules (id, branch_id, name, description, rule_type, condition_json, action, severity, is_active) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, true)",
    )
    .bind::<diesel::sql_types::Uuid, _>(&id)
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .bind::<diesel::sql_types::Text, _>(&payload.name)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&payload.description)
    .bind::<diesel::sql_types::Text, _>(&payload.rule_type)
    .bind::<diesel::sql_types::Jsonb, _>(&payload.condition)
    .bind::<diesel::sql_types::Text, _>(&payload.action)
    .bind::<diesel::sql_types::Text, _>(&payload.severity)
    .execute(&mut conn)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({"id": id})))
}

pub async fn toggle_rule(
    State(state): State<Arc<FraudState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let branch = resolve_branch(&headers);
    let mut conn = state.pool.get().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    diesel::sql_query(
        "UPDATE fraud_rules SET is_active = NOT is_active, updated_at = NOW() WHERE id = $1 AND branch_id = $2",
    )
    .bind::<diesel::sql_types::Uuid, _>(&id)
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .execute(&mut conn)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({"success": true})))
}

pub async fn list_events(
    State(state): State<Arc<FraudState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<FraudEvent>>, StatusCode> {
    let branch = resolve_branch(&headers);
    let mut conn = state.pool.get().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let events = diesel::sql_query(
        "SELECT id, branch_id, event_type, entity_type, entity_id, risk_score, risk_level, \
         triggered_rules, ml_score, action_taken, details, reviewed_by, reviewed_at, created_at \
         FROM fraud_events WHERE branch_id = $1 ORDER BY created_at DESC LIMIT 100",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load::<EventRow>(&mut conn)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .into_iter()
    .map(|r| FraudEvent {
        id: r.id,
        branch_id: r.branch_id,
        event_type: r.event_type,
        entity_type: r.entity_type,
        entity_id: r.entity_id,
        risk_score: r.risk_score,
        risk_level: r.risk_level,
        triggered_rules: r.triggered_rules.as_array().cloned().unwrap_or_default(),
        ml_score: r.ml_score,
        action_taken: r.action_taken,
        details: r.details,
        created_at: r.created_at,
    })
    .collect();
    Ok(Json(events))
}

pub async fn list_blocklist(
    State(state): State<Arc<FraudState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<FraudBlocklistEntry>>, StatusCode> {
    let branch = resolve_branch(&headers);
    let mut conn = state.pool.get().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let entries = diesel::sql_query(
        "SELECT id, branch_id, block_type, block_value, reason, expires_at, created_at \
         FROM fraud_blocklist WHERE branch_id = $1 ORDER BY created_at DESC",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load::<BlocklistRow>(&mut conn)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .into_iter()
    .map(|r| FraudBlocklistEntry {
        id: r.id,
        branch_id: r.branch_id,
        block_type: r.block_type,
        block_value: r.block_value,
        reason: r.reason,
        expires_at: r.expires_at,
        created_at: r.created_at,
    })
    .collect();
    Ok(Json(entries))
}

pub async fn add_blocklist(
    State(state): State<Arc<FraudState>>,
    headers: HeaderMap,
    Json(payload): Json<BlocklistRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let branch = resolve_branch(&headers);
    let mut conn = state.pool.get().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let id = Uuid::new_v4();
    let expires = payload.expires_in_hours.map(|h| chrono::Utc::now() + chrono::Duration::hours(h));

    diesel::sql_query(
        "INSERT INTO fraud_blocklist (id, branch_id, block_type, block_value, reason, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind::<diesel::sql_types::Uuid, _>(&id)
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .bind::<diesel::sql_types::Text, _>(&payload.block_type)
    .bind::<diesel::sql_types::Text, _>(&payload.block_value)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&payload.reason)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>, _>(&expires)
    .execute(&mut conn)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({"id": id})))
}

pub async fn remove_blocklist(
    State(state): State<Arc<FraudState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let branch = resolve_branch(&headers);
    let mut conn = state.pool.get().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    diesel::sql_query("DELETE FROM fraud_blocklist WHERE id = $1 AND branch_id = $2")
        .bind::<diesel::sql_types::Uuid, _>(&id)
        .bind::<diesel::sql_types::Uuid, _>(branch)
        .execute(&mut conn)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({"success": true})))
}
