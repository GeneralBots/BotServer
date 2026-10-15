use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use botsecurity_auth::auth_api::types::AuthenticatedUser;
use botsecurity_core::response_error::ResponseError;
use diesel::prelude::*;

use crate::automations::schedules_map;
use crate::error::IntegrationError;
use crate::providers;
use crate::scope::resolve_scope;
use crate::state::IntegrationState;

#[derive(Deserialize)]
pub struct AutomationPayload {
    pub provider: String,
    pub action: String,
    #[serde(default)]
    pub params: Value,
    pub schedule: String,
}

#[derive(diesel::QueryableByName, serde::Serialize)]
pub struct AutomationRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub bot_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub provider_slug: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub action_key: String,
    #[diesel(sql_type = diesel::sql_types::Jsonb)]
    pub params: Value,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub schedule: String,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    pub enabled: bool,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
    pub last_run_at: Option<chrono::DateTime<chrono::Utc>>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub last_outcome: Option<String>,
}

fn error_response(status: StatusCode, error: IntegrationError) -> ResponseError {
    ResponseError::new((status, Json(json!({ "detail": error.to_string() }))).into_response())
}

fn parse_bot(value: &str) -> Result<Uuid, ResponseError> {
    Uuid::parse_str(value)
        .map_err(|_| error_response(StatusCode::BAD_REQUEST, IntegrationError::Validation("bot_id must be a UUID".to_string())))
}

fn validate_payload(payload: &AutomationPayload) -> Result<(), ResponseError> {
    if payload.provider.len() > 100 || payload.action.len() > 200 {
        return Err(error_response(StatusCode::BAD_REQUEST, IntegrationError::Validation(
            "provider or action exceeds the allowed length".to_string(),
        )));
    }
    if !payload.params.is_object() {
        return Err(error_response(StatusCode::BAD_REQUEST, IntegrationError::Validation(
            "params must be a JSON object".to_string(),
        )));
    }
    let allowed = schedules_map();
    if !allowed.contains_key(payload.schedule.as_str()) {
        return Err(error_response(StatusCode::BAD_REQUEST, IntegrationError::Validation(
            format!("schedule must be one of {}", allowed.keys().cloned().collect::<Vec<_>>().join(", ")),
        )));
    }
    let implemented = providers::implemented_action_names(&payload.provider);
    if implemented.is_empty() || !implemented.contains(&payload.action.as_str()) {
        return Err(error_response(StatusCode::NOT_FOUND, IntegrationError::NotFound));
    }
    Ok(())
}

/// GET /api/bots/:bot_id/integration-automations
pub async fn list(
    State(state): State<Arc<IntegrationState>>,
    user: AuthenticatedUser,
    Path(bot_id): Path<String>,
) -> Result<Response, ResponseError> {
    let uuid = parse_bot(&bot_id)?;
    let scope = resolve_scope(&state.pool, &user, uuid).map_err(|e| error_response(StatusCode::FORBIDDEN, e))?;
    let mut conn = state.pool.get().map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, IntegrationError::VaultUnavailable))?;
    let rows: Vec<AutomationRow> = diesel::sql_query(
        "SELECT id, bot_id, provider_slug, action_key, params, schedule, enabled, last_run_at, last_outcome \
         FROM integration_automations \
         WHERE org_id = $1 AND branch_id = $2 AND bot_id = $3 AND owner_user_id = $4 \
         ORDER BY created_at DESC LIMIT 500",
    )
    .bind::<diesel::sql_types::Uuid, _>(scope.org_id)
    .bind::<diesel::sql_types::Uuid, _>(scope.branch_id)
    .bind::<diesel::sql_types::Uuid, _>(scope.bot_id)
    .bind::<diesel::sql_types::Uuid, _>(scope.owner_user_id())
    .load(&mut conn)
    .map_err(|error| {
        log::error!("automation list failed: {error:?}");
        error_response(StatusCode::INTERNAL_SERVER_ERROR, IntegrationError::VaultUnavailable)
    })?;
    Ok(Json(json!({ "automations": rows })).into_response())
}

/// POST /api/bots/:bot_id/integration-automations
pub async fn create(
    State(state): State<Arc<IntegrationState>>,
    user: AuthenticatedUser,
    Path(bot_id): Path<String>,
    Json(payload): Json<AutomationPayload>,
) -> Result<Response, ResponseError> {
    validate_payload(&payload)?;
    let uuid = parse_bot(&bot_id)?;
    let scope = resolve_scope(&state.pool, &user, uuid).map_err(|e| error_response(StatusCode::FORBIDDEN, e))?;
    let mut conn = state.pool.get().map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, IntegrationError::VaultUnavailable))?;
    let id = Uuid::now_v7();
    diesel::sql_query(
        "INSERT INTO integration_automations \
         (id, org_id, branch_id, bot_id, owner_user_id, provider_slug, action_key, params, schedule) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind::<diesel::sql_types::Uuid, _>(id)
    .bind::<diesel::sql_types::Uuid, _>(scope.org_id)
    .bind::<diesel::sql_types::Uuid, _>(scope.branch_id)
    .bind::<diesel::sql_types::Uuid, _>(scope.bot_id)
    .bind::<diesel::sql_types::Uuid, _>(scope.owner_user_id())
    .bind::<diesel::sql_types::Text, _>(payload.provider.as_str())
    .bind::<diesel::sql_types::Text, _>(payload.action.as_str())
    .bind::<diesel::sql_types::Jsonb, _>(&payload.params)
    .bind::<diesel::sql_types::Text, _>(payload.schedule.as_str())
    .execute(&mut conn)
    .map_err(|error| {
        log::error!("automation insert failed: {error:?}");
        error_response(StatusCode::INTERNAL_SERVER_ERROR, IntegrationError::VaultUnavailable)
    })?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))).into_response())
}

/// DELETE /api/bots/:bot_id/integration-automations/:automation_id
pub async fn remove(
    State(state): State<Arc<IntegrationState>>,
    user: AuthenticatedUser,
    Path((bot_id, automation_id)): Path<(String, String)>,
) -> Result<Response, ResponseError> {
    let uuid = parse_bot(&bot_id)?;
    let scope = resolve_scope(&state.pool, &user, uuid).map_err(|e| error_response(StatusCode::FORBIDDEN, e))?;
    let automation = Uuid::parse_str(&automation_id)
        .map_err(|_| error_response(StatusCode::BAD_REQUEST, IntegrationError::Validation("automation_id must be a UUID".to_string())))?;
    let mut conn = state.pool.get().map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, IntegrationError::VaultUnavailable))?;
    diesel::sql_query(
        "DELETE FROM integration_automations \
         WHERE id = $5 AND org_id = $1 AND branch_id = $2 AND bot_id = $3 AND owner_user_id = $4",
    )
    .bind::<diesel::sql_types::Uuid, _>(scope.org_id)
    .bind::<diesel::sql_types::Uuid, _>(scope.branch_id)
    .bind::<diesel::sql_types::Uuid, _>(scope.bot_id)
    .bind::<diesel::sql_types::Uuid, _>(scope.owner_user_id())
    .bind::<diesel::sql_types::Uuid, _>(automation)
    .execute(&mut conn)
    .map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, IntegrationError::VaultUnavailable))?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// PATCH /api/bots/:bot_id/integration-automations/:automation_id
#[derive(Deserialize)]
pub struct TogglePayload {
    pub enabled: bool,
}

pub async fn toggle(
    State(state): State<Arc<IntegrationState>>,
    user: AuthenticatedUser,
    Path((bot_id, automation_id)): Path<(String, String)>,
    Json(payload): Json<TogglePayload>,
) -> Result<Response, ResponseError> {
    let uuid = parse_bot(&bot_id)?;
    let scope = resolve_scope(&state.pool, &user, uuid).map_err(|e| error_response(StatusCode::FORBIDDEN, e))?;
    let automation = Uuid::parse_str(&automation_id)
        .map_err(|_| error_response(StatusCode::BAD_REQUEST, IntegrationError::Validation("automation_id must be a UUID".to_string())))?;
    let mut conn = state.pool.get().map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, IntegrationError::VaultUnavailable))?;
    diesel::sql_query(
        "UPDATE integration_automations SET enabled = $5, updated_at = NOW(), last_run_at = NULL \
         WHERE id = $6 AND org_id = $1 AND branch_id = $2 AND bot_id = $3 AND owner_user_id = $4",
    )
    .bind::<diesel::sql_types::Uuid, _>(scope.org_id)
    .bind::<diesel::sql_types::Uuid, _>(scope.branch_id)
    .bind::<diesel::sql_types::Uuid, _>(scope.bot_id)
    .bind::<diesel::sql_types::Uuid, _>(scope.owner_user_id())
    .bind::<diesel::sql_types::Bool, _>(payload.enabled)
    .bind::<diesel::sql_types::Uuid, _>(automation)
    .execute(&mut conn)
    .map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, IntegrationError::VaultUnavailable))?;
    Ok(Json(json!({ "enabled": payload.enabled })).into_response())
}
