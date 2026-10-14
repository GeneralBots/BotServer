use axum::extract::{Json, Path};
use axum::http::{HeaderMap, StatusCode};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use diesel::RunQueryDsl;
use diesel::OptionalExtension;

use crate::db;
use crate::storage;

/// Resolves the caller's tenant branch from the server-minted JWT claims
/// (issue #734). Falls back to the global nil branch for anonymous/system
/// callers; every query is still bounded by the resolved branch.
fn resolve_branch(headers: &HeaderMap) -> Uuid {
    botcore::shared::tenant::branch_from_claims(headers).unwrap_or_else(Uuid::nil)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharePointItem {
    pub id: Uuid,
    pub site_name: String,
    pub list_name: String,
    pub item_count: i64,
    pub last_modified: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: Uuid,
    pub subject: String,
    pub start: chrono::DateTime<Utc>,
    pub end: chrono::DateTime<Utc>,
    pub location: Option<String>,
    pub attendees: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneDriveFile {
    pub id: Uuid,
    pub name: String,
    pub path: String,
    pub size_bytes: i64,
    pub last_modified: chrono::DateTime<Utc>,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M365Settings {
    pub tenant_id: String,
    pub client_id: String,
    pub connected: bool,
    pub scopes: Vec<String>,
    pub last_sync: Option<chrono::DateTime<Utc>>,
}

pub async fn list_sharepoint(headers: HeaderMap) -> Result<Json<Vec<SharePointItem>>, (StatusCode, String)> {
    storage::ensure_schema_sync()?;
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)] site_name: String,
        #[diesel(sql_type = diesel::sql_types::Text)] list_name: String,
        #[diesel(sql_type = diesel::sql_types::BigInt)] item_count: i64,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)] last_modified: chrono::DateTime<Utc>,
    }
    let rows: Vec<Row> = diesel::sql_query(
        "SELECT id, COALESCE(site_id, '') AS site_name, COALESCE(list_id, '') AS list_name, \
         (CASE WHEN fields IS NULL THEN 0 ELSE 1 END)::bigint AS item_count, \
         COALESCE(modified_at, synced_at) AS last_modified
         FROM m365_sharepoint_items WHERE branch_id = $1 ORDER BY synced_at DESC LIMIT 500",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .map_err(db::map_diesel_err)?;
    Ok(Json(rows.into_iter().map(|r| SharePointItem {
        id: r.id, site_name: r.site_name, list_name: r.list_name,
        item_count: r.item_count, last_modified: r.last_modified,
    }).collect()))
}

pub async fn list_calendar(headers: HeaderMap) -> Result<Json<Vec<CalendarEvent>>, (StatusCode, String)> {
    storage::ensure_schema_sync()?;
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)] subject: String,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)] start_time: chrono::DateTime<Utc>,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)] end_time: chrono::DateTime<Utc>,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)] location: Option<String>,
        #[diesel(sql_type = diesel::sql_types::Jsonb)] attendees: serde_json::Value,
        #[diesel(sql_type = diesel::sql_types::Text)] status: String,
    }
    let rows: Vec<Row> = diesel::sql_query(
        "SELECT id, subject, start_time, end_time, location, attendees, status
         FROM m365_calendar_events WHERE branch_id = $1 ORDER BY start_time DESC LIMIT 500",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .map_err(db::map_diesel_err)?;
    Ok(Json(rows.into_iter().map(|r| CalendarEvent {
        id: r.id, subject: r.subject, start: r.start_time, end: r.end_time,
        location: r.location,
        attendees: r.attendees.as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default(),
        status: r.status,
    }).collect()))
}

/// List the caller's mirrored OneDrive files.
///
/// This used to read the never-populated `m365_onedrive_files` table and
/// always answer `[]`. It now reads the mirror the real sync engine maintains
/// (`botdrive::external`), so a connected account actually lists its files.
pub async fn list_onedrive(headers: HeaderMap) -> Result<Json<Vec<OneDriveFile>>, (StatusCode, String)> {
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool
        .get()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    let rows = botdrive::external::files::list_files(
        &mut conn,
        branch,
        Some(botdrive::external::Provider::OneDrive),
        None,
        500,
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(
        rows.into_iter()
            .map(|r| OneDriveFile {
                id: r.id,
                name: r.name,
                path: r.path,
                size_bytes: r.size_bytes,
                // Graph's delta feed reports no author; the field is part of
                // the published shape, so it stays empty rather than invented.
                last_modified: r.modified_at.unwrap_or_else(Utc::now),
                author: String::new(),
            })
            .collect(),
    ))
}

pub async fn get_settings(headers: HeaderMap) -> Result<Json<Option<M365Settings>>, (StatusCode, String)> {
    storage::ensure_schema_sync()?;
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Text)] tenant_id: String,
        #[diesel(sql_type = diesel::sql_types::Text)] client_id: String,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)] last_sync: Option<chrono::DateTime<Utc>>,
    }
    let row: Option<Row> = diesel::sql_query(
        "SELECT tenant_id, client_id, last_sync FROM oauth_microsoft_settings \
         WHERE branch_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .get_result(&mut conn)
    .optional()
    .map_err(db::map_diesel_err)?;
    Ok(Json(row.map(|r| M365Settings {
        tenant_id: r.tenant_id,
        client_id: r.client_id,
        connected: r.last_sync.is_some(),
        scopes: Vec::new(),
        last_sync: r.last_sync,
    })))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M365Team {
    pub id: Uuid,
    pub display_name: String,
    pub description: String,
    pub channel_count: i64,
    pub member_count: i64,
}

pub async fn list_teams(headers: HeaderMap) -> Result<Json<Vec<M365Team>>, (StatusCode, String)> {
    storage::ensure_schema_sync()?;
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)] display_name: String,
        #[diesel(sql_type = diesel::sql_types::Text)] description: String,
        #[diesel(sql_type = diesel::sql_types::BigInt)] channel_count: i64,
        #[diesel(sql_type = diesel::sql_types::BigInt)] member_count: i64,
    }
    let rows: Vec<Row> = diesel::sql_query(
        "SELECT id, display_name, description, channel_count, member_count
         FROM m365_teams WHERE branch_id = $1 ORDER BY display_name ASC LIMIT 500",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .map_err(db::map_diesel_err)?;
    Ok(Json(rows.into_iter().map(|r| M365Team {
        id: r.id, display_name: r.display_name, description: r.description,
        channel_count: r.channel_count, member_count: r.member_count,
    }).collect()))
}

use axum::body::Body;
use axum::http::{header, HeaderValue, StatusCode as HttpStatusCode};
use axum::response::Response;

/// Download the real bytes of a mirrored OneDrive file.
///
/// Previously this returned a hand-written text description of a row that no
/// code ever wrote. It now streams the file from Microsoft Graph, using the
/// same tenant-scoped mirror as the listing.
pub async fn download_file(headers: HeaderMap, Path(id): Path<String>) -> Result<Response<Body>, (StatusCode, String)> {
    let branch = resolve_branch(&headers);
    let parsed = Uuid::parse_str(&id).map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid file id '{id}': {e}")))?;
    let provider = botdrive::external::Provider::OneDrive;
    let pool = db::pool()?;
    let (row, connection) = {
        let mut conn = pool
            .get()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
        let row = botdrive::external::files::get_file_by_id(&mut conn, branch, provider, parsed)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
            .ok_or((StatusCode::NOT_FOUND, format!("File {id} not found")))?;
        let connection = botdrive::external::db::get_connection(&mut conn, branch, provider)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
            .ok_or((StatusCode::CONFLICT, "OneDrive is not connected".to_string()))?;
        (row, connection)
    };

    let http = botdrive::external::engine::http_client()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let token = botdrive::external::engine::connection_access_token(&http, pool, branch, &connection)
        .await
        .map_err(|e| match e {
            botdrive::external::SyncError::Auth(_) => {
                (StatusCode::UNAUTHORIZED, "reconnect OneDrive".to_string())
            }
            other => (StatusCode::BAD_GATEWAY, other.to_string()),
        })?;
    let bytes = botdrive::external::onedrive::download(&http, &token, &row.remote_id)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let safe_name = row.name.replace(['/', '\\', '"', '\n', '\r'], "_");
    let cd = format!("attachment; filename=\"{}\"", safe_name);
    Response::builder()
        .status(HttpStatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            row.mime_type.unwrap_or_else(|| "application/octet-stream".to_string()),
        )
        .header(header::CONTENT_DISPOSITION, HeaderValue::from_str(&cd).unwrap_or_else(|_| HeaderValue::from_static("attachment")))
        .body(Body::from(bytes))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Response error: {e}")))
}

/// Start a real OneDrive consent flow.
///
/// This used to insert a settings row and answer `{"connected": true}` without
/// ever contacting Microsoft, so the UI showed a connected account that could
/// not sync anything. It now returns the provider authorization URL (or a
/// clear 501 when the deployment has no OAuth client registered).
pub async fn connect_account(headers: HeaderMap) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let branch = resolve_branch(&headers);
    let provider = botdrive::external::Provider::OneDrive;
    let client = botdrive::external::config::client_for(provider).ok_or((
        StatusCode::NOT_IMPLEMENTED,
        "no Microsoft OAuth client is configured for this deployment".to_string(),
    ))?;
    let base = botdrive::external::config::base_from_headers(
        &headers,
        None,
        headers.get("x-forwarded-proto").and_then(|v| v.to_str().ok()),
    );
    let redirect_uri = botdrive::external::config::redirect_uri(base.as_deref());
    let state = botdrive::external::crypto::state_token(branch, provider);
    let authorize_url = botdrive::external::onedrive::authorize_url(
        &client.tenant_id,
        &client.client_id,
        &redirect_uri,
        &state,
    );
    Ok(Json(serde_json::json!({
        "connected": false,
        "authorize_url": authorize_url,
        "redirect_uri": redirect_uri,
    })))
}

pub async fn disconnect_account(headers: HeaderMap) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    storage::ensure_schema_sync()?;
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    diesel::sql_query("UPDATE oauth_microsoft_settings SET connected_at = NULL, updated_at = NOW() WHERE branch_id = $1")
        .bind::<diesel::sql_types::Uuid, _>(branch)
        .execute(&mut conn)
        .map_err(db::map_diesel_err)?;
    Ok(Json(serde_json::json!({ "connected": false })))
}

/// Run a real OneDrive sync pass.
///
/// This used to only stamp `last_sync = NOW()` on a settings row and report
/// success — nothing was ever fetched. It now drives the Graph delta feed
/// through the shared external-drive engine and returns what actually changed.
pub async fn sync_now(headers: HeaderMap) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let report = botdrive::external::engine::sync_connection(
        pool,
        branch,
        botdrive::external::Provider::OneDrive,
    )
    .await;
    if let Some(err) = report.error.as_deref() {
        log::warn!("m365 onedrive sync reported {err}");
    }
    Ok(Json(serde_json::json!({
        "synced_at": Utc::now(),
        "upserted": report.upserted,
        "removed": report.removed,
        "pages": report.pages,
        "error": report.error,
        "error_kind": report.error_kind,
    })))
}

pub async fn update_sync_settings(headers: HeaderMap, Json(req): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    storage::ensure_schema_sync()?;
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    let sync_type = req.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let frequency = req.get("frequency").and_then(|v| v.as_i64()).unwrap_or(15);
    let col = if sync_type == "calendar" { "sync_calendar_min" } else { "sync_onedrive_min" };
    diesel::sql_query(format!(
        "UPDATE oauth_microsoft_settings SET {col} = $1, updated_at = NOW() WHERE branch_id = $2"
    ))
    .bind::<diesel::sql_types::BigInt, _>(frequency)
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .execute(&mut conn)
    .map_err(db::map_diesel_err)?;
    Ok(Json(serde_json::json!({ "type": sync_type, "frequency": frequency })))
}
