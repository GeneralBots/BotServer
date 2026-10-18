//! Workspace tab persistence for the suite chat (issue #1168).
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use botcore::shared::state::AppState;

type HandlerError = (StatusCode, String);

const MAX_TABS: usize = 20;
const MAX_TAB_JSON_BYTES: usize = 32 * 1024;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::user_workspace_tabs;

#[derive(Debug, Clone, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = user_workspace_tabs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct WorkspaceTabsRow {
    user_id: Uuid,
    tabs: serde_json::Value,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct TabsBody {
    tabs: serde_json::Value,
}

fn requester(headers: &HeaderMap, state: &Arc<AppState>) -> Result<Uuid, HandlerError> {
    let raw = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or_default();
    if raw.is_empty() {
        return Err((StatusCode::UNAUTHORIZED, "Authentication required".to_string()));
    }
    let sub = jwt_subject(raw)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Invalid token".to_string()))?;
    resolve_user_uuid(state, &sub)
}

/// Minimal unverified extraction of the `sub` claim for user resolution.
/// Signature verification is delegated to the auth middleware upstream.
fn jwt_subject(token: &str) -> Option<String> {
    use base64::Engine as _;
    let payload_b64 = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(payload_b64))
        .ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    value["sub"].as_str().map(|s| s.to_string())
}

/// Resolves the platform user row from a JWT subject claim. Subjects minted
/// by the directory provider map through UUIDv5; numeric/anonymous ids pass
/// through `resolve_chat_user_uuid` helpers used elsewhere.
fn resolve_user_uuid(state: &Arc<AppState>, subject: &str) -> Result<Uuid, HandlerError> {
    let derived = uuid::Uuid::new_v5(&uuid::Uuid::nil(), format!("zitadel:{subject}").as_bytes());
    let mut conn = state
        .conn
        .get()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB pool: {e}")))?;
    let found: Option<Uuid> = diesel::sql_query("SELECT id AS user_id FROM users WHERE id = $1")
        .bind::<diesel::sql_types::Uuid, _>(derived)
        .get_result::<UserIdRow>(&mut conn)
        .map(|r| r.user_id)
        .optional()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("user lookup: {e}")))?;
    Ok(found.unwrap_or(derived))
}

#[derive(diesel::QueryableByName)]
struct UserIdRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    user_id: Uuid,
}

pub async fn get_tabs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, HandlerError> {
    let user_id = requester(&headers, &state)?;
    let mut conn = state
        .conn
        .get()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB pool: {e}")))?;
    let row = user_workspace_tabs::dsl::user_workspace_tabs
        .find(user_id)
        .select(WorkspaceTabsRow::as_select())
        .get_result::<WorkspaceTabsRow>(&mut conn)
        .optional()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("tabs lookup: {e}")))?;
    match row {
        Some(r) => Ok(Json(serde_json::json!({
            "tabs": r.tabs,
            "updated_at": r.updated_at.to_rfc3339(),
        }))),
        None => Ok(Json(serde_json::json!({ "tabs": [], "updated_at": null }))),
    }
}

pub async fn put_tabs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<TabsBody>,
) -> Result<Json<serde_json::Value>, HandlerError> {
    let user_id = requester(&headers, &state)?;
    if serde_json::to_string(&body.tabs)
        .map(|s| s.len() > MAX_TAB_JSON_BYTES)
        .unwrap_or(true)
    {
        return Err((StatusCode::BAD_REQUEST, "tabs payload too large".to_string()));
    }
    if body.tabs.as_array().map(|a| a.len()).unwrap_or(0) > MAX_TABS {
        return Err((StatusCode::BAD_REQUEST, "too many tabs".to_string()));
    }
    let row = WorkspaceTabsRow {
        user_id,
        tabs: body.tabs.clone(),
        updated_at: Utc::now(),
    };
    let mut conn = state
        .conn
        .get()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB pool: {e}")))?;
    diesel::insert_into(user_workspace_tabs::dsl::user_workspace_tabs)
        .values(&row)
        .on_conflict(user_workspace_tabs::dsl::user_id)
        .do_update()
        .set((
            user_workspace_tabs::dsl::tabs.eq(row.tabs.clone()),
            user_workspace_tabs::dsl::updated_at.eq(row.updated_at),
        ))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("tabs save: {e}")))?;
    Ok(Json(serde_json::json!({
        "saved": true,
        "updated_at": row.updated_at.to_rfc3339(),
    })))
}

pub fn configure_workspace_tabs_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/user/workspace/tabs", get(get_tabs).put(put_tabs))
}
