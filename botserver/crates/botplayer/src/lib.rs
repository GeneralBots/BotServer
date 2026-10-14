//! Media player backend: drive streaming plus persisted playlists.
//!
//! Streaming routes read media bytes from the bot's MinIO drive bucket
//! (`{bot_id}.gbai/{bot_id}.gbdrive/...`) through `AppState.drive`. Playlist
//! routes persist playlists/items and playback analytics in Postgres via
//! `AppState.conn`.

use async_trait::async_trait;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use botcore::shared::state::AppState;

pub mod playlists;

/// Byte-level access to drive storage used by the streaming handlers.
#[async_trait]
pub trait DriveStore: Send + Sync {
    async fn get_object_bytes(&self, bucket: &str, key: &str) -> Result<Vec<u8>, String>;
}

/// Reads an object from the app's drive repository. Returns `None` when no
/// drive backend is configured (suite without drive) so handlers can answer
/// 503 instead of panicking.
async fn drive_bytes(state: &Arc<AppState>, bucket: &str, key: &str) -> Option<Vec<u8>> {
    let drive = state.drive.as_ref()?;
    drive.get_object(bucket, key).await.ok()
}

/// Drive location for the bot named in the request path.
///
/// An org-hosted bot lives in `{slug}.gborg` with the bot directory as a key
/// prefix, so `{bot}.gbai` + `{bot}.gbdrive/{path}` reads an empty bucket.
fn bot_location(
    state: &Arc<AppState>,
    bot_name: &str,
) -> botbasic_core::utils::BotDriveLocation {
    let mut conn = match state.conn.get() {
        Ok(conn) => conn,
        Err(e) => {
            log::error!("player: DB unavailable resolving drive location: {e}");
            return botbasic_core::utils::resolve_bot_drive_location(bot_name, None, None);
        }
    };
    botbasic_core::utils::bot_drive_location_for_name(&mut conn, bot_name)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInfo {
    pub path: String,
    pub filename: String,
    pub mime_type: String,
    pub size: u64,
    pub duration: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailInfo {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub format: String,
}

#[derive(Debug, Deserialize)]
pub struct StreamQuery {
    pub quality: Option<String>,
    pub start: Option<f64>,
    pub end: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct ThumbnailQuery {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub time: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct PlayerError {
    pub error: String,
    pub code: String,
}

impl IntoResponse for PlayerError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": self.error, "code": self.code})),
        )
            .into_response()
    }
}

fn get_mime_type(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "ogv" => "video/ogg",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "m4a" => "audio/mp4",
        "flac" => "audio/flac",
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

fn get_format(path: &str) -> String {
    path.rsplit('.')
        .next()
        .unwrap_or("unknown")
        .to_uppercase()
}

async fn get_file_info(
    State(state): State<Arc<AppState>>,
    Path((bot_id, path)): Path<(String, String)>,
) -> Result<Json<MediaInfo>, PlayerError> {
    let filename = path.rsplit('/').next().unwrap_or(&path).to_string();
    let mime_type = get_mime_type(&path).to_string();
    let format = get_format(&path);

    // Fetch the object so `size` reflects the real file. Missing files
    // surface an honest error instead of a fake zero-byte entry.
    let loc = bot_location(&state, &bot_id);
    let full_path = loc.key_for(&path);
    let bytes = drive_bytes(&state, &loc.bucket, &full_path)
        .await
        .ok_or_else(|| PlayerError {
            error: "Failed to get file".to_string(),
            code: "FILE_NOT_FOUND".to_string(),
        })?;

    let info = MediaInfo {
        path: format!("{bot_id}/{path}"),
        filename,
        mime_type,
        size: bytes.len() as u64,
        duration: None,
        width: None,
        height: None,
        format,
    };

    Ok(Json(info))
}

async fn stream_file(
    State(state): State<Arc<AppState>>,
    Path((bot_id, path)): Path<(String, String)>,
    Query(_query): Query<StreamQuery>,
) -> Result<Response<Body>, PlayerError> {
    let mime_type = get_mime_type(&path);
    let loc = bot_location(&state, &bot_id);
    let full_path = loc.key_for(&path);

    let bytes = drive_bytes(&state, &loc.bucket, &full_path)
        .await
        .ok_or_else(|| PlayerError {
            error: "Failed to get file".to_string(),
            code: "FILE_NOT_FOUND".to_string(),
        })?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime_type)
        .header(header::ACCEPT_RANGES, "bytes")
        .body(Body::from(bytes))
        .map_err(|e| PlayerError {
            error: botsecurity_core::error_sanitizer::log_and_sanitize(&e, "botplayer::lib", None).message,
            code: "RESPONSE_ERROR".to_string(),
        })?;

    Ok(response)
}

async fn get_thumbnail(
    State(_state): State<Arc<AppState>>,
    Path((_bot_id, path)): Path<(String, String)>,
    Query(query): Query<ThumbnailQuery>,
) -> Result<Response<Body>, PlayerError> {
    let width = query.width.unwrap_or(320);
    let height = query.height.unwrap_or(180);

    let filename = path.rsplit('/').next().unwrap_or(&path);
    let placeholder = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">
<rect width="100%" height="100%" fill="#374151"/>
<text x="50%" y="50%" text-anchor="middle" dy="0.3em" fill="#9CA3AF" font-family="sans-serif" font-size="14">
{filename}
</text>
</svg>"##
    );

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/svg+xml")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(Body::from(placeholder))
        .map_err(|e| PlayerError {
            error: botsecurity_core::error_sanitizer::log_and_sanitize(&e, "botplayer::lib", None).message,
            code: "RESPONSE_ERROR".to_string(),
        })?;

    Ok(response)
}

async fn get_supported_formats(
    State(_state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "video": ["mp4", "webm", "ogv"],
        "audio": ["mp3", "wav", "ogg", "m4a", "flac"],
        "document": ["pdf", "txt", "md", "html"],
        "image": ["png", "jpg", "jpeg", "gif", "svg", "webp"],
        "presentation": ["pptx", "odp"]
    }))
}

/// Builds the full player router (streaming + playlists) with `AppState`.
pub fn configure_player_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/player/formats", get(get_supported_formats))
        .route("/api/player/:bot_id/info/*path", get(get_file_info))
        .route("/api/player/:bot_id/stream/*path", get(stream_file))
        .route("/api/player/:bot_id/thumbnail/*path", get(get_thumbnail))
        .route(
            "/api/player/playlists",
            get(playlists::list_playlists).post(playlists::create_playlist),
        )
        .route(
            "/api/player/playlists/:id",
            get(playlists::get_playlist)
                .patch(playlists::update_playlist)
                .delete(playlists::delete_playlist),
        )
        .route(
            "/api/player/playlists/:id/items",
            post(playlists::add_item),
        )
        .route(
            "/api/player/playlists/:id/items/:item_id",
            delete(playlists::remove_item),
        )
        .route(
            "/api/player/playlists/:id/reorder",
            put(playlists::reorder_items),
        )
        .route(
            "/api/player/playlists/:id/analytics",
            get(playlists::playlist_analytics),
        )
        .route("/api/player/playbacks", post(playlists::record_playback))
}
