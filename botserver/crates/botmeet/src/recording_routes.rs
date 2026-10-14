//! HTTP surface for meeting recording.
//!
//! Exposes the `RecordingService` (recording.rs) through REST endpoints:
//! start/stop/pause/resume for a room, plus list/get/delete and file
//! streaming for completed recordings. Recording controls are restricted to
//! hosts and co-hosts via the `meeting_participants` role column.

use axum::{
    extract::{Path, State},
    http::{header, HeaderValue, StatusCode},
    response::Response,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use botcore::shared::state::AppState;

use crate::recording::{
    RecordingConfig, RecordingError, RecordingService, StartRecordingRequest, StopRecordingRequest,
};
use crate::webinar_types::{RecordingStatus, WebinarRecording};

/// Roles allowed to control recording.
const HOST_ROLES: &[&str] = &["host", "co_host", "co-host", "presenter"];

fn recording_error(e: RecordingError) -> (StatusCode, Json<serde_json::Value>) {
    let (status, message) = match &e {
        RecordingError::NotFound => (StatusCode::NOT_FOUND, e.to_string()),
        RecordingError::AlreadyRecording | RecordingError::AlreadyExists => {
            (StatusCode::CONFLICT, e.to_string())
        }
        RecordingError::Unauthorized => (StatusCode::FORBIDDEN, e.to_string()),
        RecordingError::InvalidState(_) | RecordingError::TranscriptionNotReady => {
            (StatusCode::CONFLICT, e.to_string())
        }
        _ => {
            log::error!("Meet recording error: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
    };
    (status, Json(serde_json::json!({ "error": message })))
}

/// Shared `RecordingService` instance.
///
/// The service keeps in-memory active sessions (start → pause/stop must hit
/// the same instance), so a single process-wide instance is created lazily
/// from the first request's pool. All requests share it; the DB row is the
/// source of truth for the recording lifecycle.
static RECORDING_SERVICE: std::sync::OnceLock<RecordingService> = std::sync::OnceLock::new();

fn service_for(state: &Arc<AppState>) -> &'static RecordingService {
    RECORDING_SERVICE.get_or_init(|| {
        RecordingService::new(state.conn.clone(), RecordingConfig::default())
    })
}

/// Resolves the participant role for a room; returns `None` when the user is
/// not (or no longer) a member of the room.
async fn participant_role(
    state: &Arc<AppState>,
    target_room: Uuid,
    target_user: Uuid,
) -> Result<Option<String>, (StatusCode, Json<serde_json::Value>)> {
    use diesel::prelude::*;
    use botschema::meeting_participants::dsl::{
        meeting_participants, role as role_col, room_id, user_id,
    };

    let pool = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| format!("Failed to acquire connection: {e}"))?;
        meeting_participants
            .filter(room_id.eq(target_room))
            .filter(user_id.eq(target_user))
            .select(role_col)
            .first::<String>(&mut conn)
            .optional()
            .map_err(|e| format!("Failed to query participant role: {e}"))
    })
    .await;

    match result {
        Ok(Ok(role)) => Ok(role),
        Ok(Err(e)) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": botsecurity::error_sanitizer::log_and_sanitize_json(&e.to_string(), "botmeet::recording_routes", None) })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": botsecurity::error_sanitizer::log_and_sanitize_json(&e.to_string(), "botmeet::recording_routes", None) })),
        )),
    }
}

/// Verifies the caller is a host/co-host of the room; returns 403 otherwise.
async fn require_host(
    state: &Arc<AppState>,
    room_id: Uuid,
    user_id: Uuid,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    match participant_role(state, room_id, user_id).await? {
        Some(role) if HOST_ROLES.contains(&role.as_str()) => Ok(()),
        Some(_) => Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "Only the host or co-host can control recording" })),
        )),
        None => Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "Not a participant of this meeting" })),
        )),
    }
}

/// JSON body carrying the acting user id (for RBAC) next to the recording
/// payload. Kept separate from the service request types so the service layer
/// stays auth-agnostic.
#[derive(Debug, Deserialize)]
pub struct RecordingControlRequest<T> {
    pub user_id: Uuid,
    #[serde(flatten)]
    pub payload: T,
}

/// `POST /api/meet/rooms/{id}/recording/start`
pub async fn start_recording(
    State(state): State<Arc<AppState>>,
    Path(room_id): Path<Uuid>,
    Json(body): Json<RecordingControlRequest<StartRecordingRequest>>,
) -> Result<Json<WebinarRecording>, (StatusCode, Json<serde_json::Value>)> {
    require_host(&state, room_id, body.user_id).await?;

    let mut request = body.payload;
    request.webinar_id = room_id;
    service_for(&state)
        .start_recording(request)
        .await
        .map(Json)
        .map_err(recording_error)
}

/// `POST /api/meet/rooms/{id}/recording/stop`
pub async fn stop_recording(
    State(state): State<Arc<AppState>>,
    Path(room_id): Path<Uuid>,
    Json(body): Json<RecordingControlRequest<StopRecordingRequest>>,
) -> Result<Json<WebinarRecording>, (StatusCode, Json<serde_json::Value>)> {
    require_host(&state, room_id, body.user_id).await?;

    service_for(&state)
        .stop_recording(body.payload)
        .await
        .map(Json)
        .map_err(recording_error)
}

/// `POST /api/meet/rooms/{id}/recording/pause`
pub async fn pause_recording(
    State(state): State<Arc<AppState>>,
    Path((room_id, recording_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<RecordingControlRequest<serde_json::Value>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    require_host(&state, room_id, body.user_id).await?;

    service_for(&state)
        .pause_recording(recording_id)
        .await
        .map_err(recording_error)?;
    Ok(Json(serde_json::json!({ "success": true, "message": "Recording paused" })))
}

/// `POST /api/meet/rooms/{id}/recording/resume`
pub async fn resume_recording(
    State(state): State<Arc<AppState>>,
    Path((room_id, recording_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<RecordingControlRequest<serde_json::Value>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    require_host(&state, room_id, body.user_id).await?;

    service_for(&state)
        .resume_recording(recording_id)
        .await
        .map_err(recording_error)?;
    Ok(Json(serde_json::json!({ "success": true, "message": "Recording resumed" })))
}

/// `GET /api/meet/rooms/{id}/recordings`
pub async fn list_recordings(
    State(state): State<Arc<AppState>>,
    Path(room_id): Path<Uuid>,
) -> Result<Json<Vec<WebinarRecording>>, (StatusCode, Json<serde_json::Value>)> {
    service_for(&state)
        .list_recordings(room_id)
        .await
        .map(Json)
        .map_err(recording_error)
}

/// `GET /api/meet/recordings/{recording_id}`
pub async fn get_recording(
    State(state): State<Arc<AppState>>,
    Path(recording_id): Path<Uuid>,
) -> Result<Json<WebinarRecording>, (StatusCode, Json<serde_json::Value>)> {
    service_for(&state)
        .get_recording(recording_id)
        .await
        .map(Json)
        .map_err(recording_error)
}

/// `DELETE /api/meet/recordings/{recording_id}`
pub async fn delete_recording(
    State(state): State<Arc<AppState>>,
    Path(recording_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    service_for(&state)
        .delete_recording(recording_id)
        .await
        .map_err(recording_error)?;
    Ok(Json(serde_json::json!({ "success": true, "message": "Recording deleted" })))
}

/// `GET /api/meet/recordings/{recording_id}/file`
///
/// Streams the recording file. The object lives in MinIO drive at
/// `meet/recordings/{recording_id}.webm` (per-org bucket) when the egress has
/// uploaded it; until then the recording is still processing and a 409 is
/// returned so the UI can poll.
pub async fn get_recording_file(
    State(state): State<Arc<AppState>>,
    Path(recording_id): Path<Uuid>,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let recording = service_for(&state)
        .get_recording(recording_id)
        .await
        .map_err(recording_error)?;

    if recording.status != RecordingStatus::Ready {
        return Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "error": "Recording is not ready yet" })),
        ));
    }

    let drive = state.drive.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Drive unavailable" })),
        )
    })?;

    let bucket = state.bucket_name.clone();
    let key = format!("meet/recordings/{recording_id}.webm");
    let bytes = drive.get_object(&bucket, &key).await.map_err(|e| {
        log::error!("Failed to read recording {recording_id} from drive: {e}");
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Recording file not found" })),
        )
    })?;

    let mut response = Response::new(axum::body::Body::from(bytes));
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("video/webm"));
    response
        .headers_mut()
        .insert(header::CONTENT_DISPOSITION, HeaderValue::from_static("inline"));
    Ok(response)
}
