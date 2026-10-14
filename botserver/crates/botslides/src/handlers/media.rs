use crate::storage::{get_current_user_id, load_presentation_by_id, DriveOps};
use crate::types::{
    AddMediaRequest, DeleteMediaRequest, ListMediaResponse, UpdateMediaRequest,
};
use crate::SlidesState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;

pub async fn handle_add_media<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
    Json(req): Json<AddMediaRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();

    let drive = state.drive.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Drive not configured" })),
        )
    })?;

    let mut presentation = load_presentation_by_id(drive, &user_id, &req.presentation_id)
        .await
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e })),
            )
        })?;

    if req.slide_index >= presentation.slides.len() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid slide index" })),
        ));
    }

    let media_list = presentation.slides[req.slide_index]
        .media
        .get_or_insert_with(Vec::new);
    media_list.push(req.media.clone());
    presentation.updated_at = Utc::now();

    if let Err(e) = crate::storage::save_presentation_to_drive(drive, &user_id, &presentation)
        .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botslides::handlers::media", None)})),
        ));
    }

    Ok(Json(serde_json::json!({ "success": true, "media": req.media })))
}

pub async fn handle_update_media<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
    Json(req): Json<UpdateMediaRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();

    let drive = state.drive.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Drive not configured" })),
        )
    })?;

    let mut presentation = load_presentation_by_id(drive, &user_id, &req.presentation_id)
        .await
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e })),
            )
        })?;

    if req.slide_index >= presentation.slides.len() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid slide index" })),
        ));
    }

    if let Some(media_list) = &mut presentation.slides[req.slide_index].media {
        for media in media_list.iter_mut() {
            if media.id == req.media_id {
                if let Some(autoplay) = req.autoplay {
                    media.autoplay = autoplay;
                }
                if let Some(loop_playback) = req.loop_playback {
                    media.loop_playback = loop_playback;
                }
                if let Some(muted) = req.muted {
                    media.muted = muted;
                }
                if let Some(volume) = req.volume {
                    media.volume = Some(volume);
                }
                if let Some(start_time) = req.start_time {
                    media.start_time = Some(start_time);
                }
                if let Some(end_time) = req.end_time {
                    media.end_time = Some(end_time);
                }
                break;
            }
        }
    }

    presentation.updated_at = Utc::now();
    if let Err(e) = crate::storage::save_presentation_to_drive(drive, &user_id, &presentation)
        .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botslides::handlers::media", None)})),
        ));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn handle_delete_media<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
    Json(req): Json<DeleteMediaRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();

    let drive = state.drive.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Drive not configured" })),
        )
    })?;

    let mut presentation = load_presentation_by_id(drive, &user_id, &req.presentation_id)
        .await
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e })),
            )
        })?;

    if req.slide_index >= presentation.slides.len() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid slide index" })),
        ));
    }

    if let Some(media_list) = &mut presentation.slides[req.slide_index].media {
        media_list.retain(|m| m.id != req.media_id);
    }

    presentation.updated_at = Utc::now();
    if let Err(e) = crate::storage::save_presentation_to_drive(drive, &user_id, &presentation)
        .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botslides::handlers::media", None)})),
        ));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn handle_list_media<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ListMediaResponse>, (StatusCode, Json<serde_json::Value>)> {
    let presentation_id = params.get("presentation_id").cloned().unwrap_or_default();
    let slide_index: usize = params
        .get("slide_index")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let user_id = get_current_user_id();

    let drive = state.drive.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Drive not configured" })),
        )
    })?;

    let presentation = load_presentation_by_id(drive, &user_id, &presentation_id)
        .await
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e })),
            )
        })?;

    if slide_index >= presentation.slides.len() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid slide index" })),
        ));
    }

    let media = presentation.slides[slide_index]
        .media
        .clone()
        .unwrap_or_default();
    Ok(Json(ListMediaResponse { media }))
}
