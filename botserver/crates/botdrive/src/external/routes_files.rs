//! Listing and download handlers — the read half of the external-drive API.
//!
//! Split from [`super::routes`] so each file stays focused: that one owns the
//! connect/callback lifecycle (which writes credentials), this one only reads
//! what the sync engine already mirrored.

use crate::external::{config, db, engine, files, types::Provider};
use super::routes::{caller_branch, err, parse_provider, sanitize_filename, ApiError, ApiResult};
use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use botcore::shared::state::AppState;
use botsecurity_auth::auth_api::types::AuthenticatedUser;
use serde::Deserialize;
use std::sync::Arc;

/// Default and maximum mirrored files returned per listing.
const DEFAULT_LIMIT: i64 = 500;
const MAX_LIMIT: i64 = 5000;

/// Map a provider failure onto an HTTP status: a rejected token is the caller's
/// problem to fix (reconnect), anything else is upstream being unavailable.
fn provider_failure(e: crate::external::types::SyncError) -> ApiError {
    match e {
        crate::external::types::SyncError::Auth(_) => {
            err(StatusCode::UNAUTHORIZED, "reconnect the account")
        }
        other => err(StatusCode::BAD_GATEWAY, &other.to_string()),
    }
}

/// `GET /api/external-drives/connections` — every provider, connected or not,
/// with its configuration and connection state. Drives the External tab.
pub async fn list_connections(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
) -> ApiResult<Json<serde_json::Value>> {
    let branch_id = caller_branch(&state, &user)?;
    let mut conn = db::conn(&state.conn)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("database unavailable: {e}")))?;
    let statuses = db::list_connection_status(&mut conn, branch_id)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;

    let mut providers = Vec::new();
    for provider in Provider::ALL {
        let status = statuses.iter().find(|s| s.provider == provider.as_str());
        let connected = status.is_some_and(|s| s.status != "disconnected");
        let files = if connected {
            files::count_files(&mut conn, branch_id, provider).unwrap_or(0)
        } else {
            0
        };
        providers.push(serde_json::json!({
            "provider": provider.as_str(),
            "display_name": provider.display_name(),
            "configured": config::client_for(provider).is_some(),
            "connected": connected,
            "status": status.map(|s| s.status.as_str()).unwrap_or("not_connected"),
            "account": status.and_then(|s| s.account.clone()),
            "sync_minutes": status.map(|s| s.sync_minutes).unwrap_or(30),
            "last_sync": status.and_then(|s| s.last_sync),
            "last_error": status.and_then(|s| s.last_error.clone()),
            "file_count": files,
        }));
    }
    Ok(Json(serde_json::json!({ "providers": providers })))
}

/// `GET /api/external-drives/files` — mirrored listing for one provider.
pub async fn list_files(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<ListFilesQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let branch_id = caller_branch(&state, &user)?;
    let provider = params.provider.as_deref().map(parse_provider).transpose()?;
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let mut conn = db::conn(&state.conn)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("database unavailable: {e}")))?;
    let files = files::list_files(&mut conn, branch_id, provider, params.path.as_deref(), limit)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    let total = files.len();
    let items: Vec<serde_json::Value> = files
        .into_iter()
        .map(|f| {
            serde_json::json!({
                "provider": f.provider,
                "remote_id": f.remote_id,
                "name": f.name,
                "path": f.path,
                "mime_type": f.mime_type,
                "size_bytes": f.size_bytes,
                "is_folder": f.is_folder,
                "modified_at": f.modified_at,
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "files": items, "total": total })))
}

/// `GET /api/external-drives/download/:provider/:remote_id` — stream the bytes
/// from the provider, with the mirrored metadata as the filename.
pub async fn download_file(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path((provider, remote_id)): Path<(String, String)>,
) -> ApiResult<Response> {
    let provider = parse_provider(&provider)?;
    let branch_id = caller_branch(&state, &user)?;
    let http = engine::http_client()
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    let (row, access_token) = {
        let mut conn = db::conn(&state.conn)
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("database unavailable: {e}")))?;
        let row = files::get_file(&mut conn, branch_id, provider, &remote_id)
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "file not found in this account"))?;
        let connection = db::get_connection(&mut conn, branch_id, provider)
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?
            .ok_or_else(|| err(StatusCode::CONFLICT, "account is not connected"))?;
        drop(conn);
        let token = engine::connection_access_token(&http, &state.conn, branch_id, &connection)
            .await
            .map_err(provider_failure)?;
        (row, token)
    };

    let bytes = match provider {
        Provider::OneDrive => crate::external::onedrive::download(&http, &access_token, &remote_id).await,
        Provider::GoogleDrive => crate::external::gdrive::download(&http, &access_token, &remote_id).await,
    }
    .map_err(provider_failure)?;

    Ok((
        [
            (
                header::CONTENT_TYPE,
                row.mime_type
                    .clone()
                    .unwrap_or_else(|| "application/octet-stream".to_string()),
            ),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", sanitize_filename(&row.name)),
            ),
        ],
        bytes,
    )
        .into_response())
}

#[derive(Debug, Deserialize)]
pub struct ListFilesQuery {
    pub provider: Option<String>,
    pub path: Option<String>,
    pub limit: Option<i64>,
}
