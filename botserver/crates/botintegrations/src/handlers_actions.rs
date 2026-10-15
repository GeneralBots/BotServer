use std::sync::Arc;

use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use botsecurity_auth::auth_api::types::AuthenticatedUser;
use botsecurity_core::response_error::ResponseError;
use serde_json::Value;

use crate::providers;
use crate::request::parse_bot_id;
use crate::scope::resolve_scope;
use crate::state::IntegrationState;

fn invalid(detail: String) -> ResponseError {
    ResponseError::new(
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Validation failed", "detail": detail })),
        )
            .into_response(),
    )
}

fn static_error(status: StatusCode, message: &'static str) -> ResponseError {
    ResponseError::new((status, Json(serde_json::json!({ "error": message }))).into_response())
}

/// Maps adapter error sentinels onto HTTP responses. Only the validation
/// branch echoes a server-generated detail string back to the caller; every
/// other branch returns a static message and keeps details in logs.
fn map_error(error: String) -> ResponseError {
    if error == providers::ERR_UNKNOWN_ACTION || error == providers::ERR_ACTION_NOT_AVAILABLE {
        return static_error(StatusCode::NOT_FOUND, "Action not found for this provider");
    }
    if let Some(detail) = error.strip_prefix("invalid_request:") {
        return invalid(detail.trim().to_string());
    }
    if error == providers::ERR_VAULT_UNAVAILABLE {
        return static_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Credential store unavailable; request rejected",
        );
    }
    log::warn!("integration action invocation failed: {error}");
    static_error(StatusCode::BAD_GATEWAY, "Provider invocation failed")
}

fn required_field(body: &Value, key: &str) -> Result<String, String> {
    body.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("{key} is required"))
}

/// POST /api/bots/:bot_id/integration-actions/invoke
///
/// Executes one registered provider action against the tenant's active
/// connection. Credentials load strictly from Vault immediately before the
/// call and never appear in responses, errors or audit events.
pub async fn invoke_action(
    State(state): State<Arc<IntegrationState>>,
    user: AuthenticatedUser,
    Path(bot_id): Path<String>,
    Json(body): Json<Value>,
) -> Result<Response, ResponseError> {
    let bot_uuid = match parse_bot_id(&bot_id) {
        Ok(uuid) => uuid,
        Err(error) => {
            let detail = match error {
                crate::error::IntegrationError::Validation(reason) => reason,
                _ => "bot_id must be a UUID".to_string(),
            };
            return Err(invalid(detail));
        }
    };
    let scope = match resolve_scope(&state.pool, &user, bot_uuid) {
        Ok(scope) => scope,
        Err(error) => return Err(ResponseError::new(error.into_response())),
    };

    let provider = required_field(&body, "provider").map_err(invalid)?;
    let action = required_field(&body, "action").map_err(invalid)?;
    if provider.len() > 100 || action.len() > 100 {
        return Err(invalid(
            "provider and action must be at most 100 characters".to_string(),
        ));
    }
    let params = match body.get("params") {
        None | Some(Value::Null) => Value::Object(serde_json::Map::new()),
        Some(value) if value.is_object() => value.clone(),
        Some(_) => return Err(invalid("params must be a JSON object".to_string())),
    };

    let outcome = providers::invoke_registered(&state, &scope, &provider, &action, &params)
        .await
        .map_err(map_error)?;

    {
        let mut conn = match state.pool.get() {
            Ok(conn) => conn,
            Err(_) => {
                return Err(static_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal storage error",
                ))
            }
        };
        crate::handlers_connections::record_outcome(
            &mut conn,
            &scope,
            None,
            "action.invoke",
            "ok",
            "low",
            &serde_json::json!({
                "provider": provider,
                "status": "ok",
                "outcome_detail": action,
            }),
        );
    }

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "provider": provider,
            "action": action,
            "summary": outcome.summary,
            "data": outcome.data,
            "truncated": outcome.truncated,
        })),
    )
        .into_response())
}
