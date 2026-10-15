use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use std::sync::Arc;

use uuid::Uuid;

use crate::engine::CaptionRequest;
use crate::engine::VideoEngine;
use crate::requests::*;
use crate::routes::AppState;
use crate::safe_error::SafeErrorResponse;

pub async fn transcribe_handler(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    axum::Json(req): axum::Json<TranscribeRequest>,
) -> impl IntoResponse {
    let engine = VideoEngine::new(state.conn.clone());
    match engine
        .transcribe_audio(project_id, req.clip_id, req.language)
        .await
    {
        Ok(transcription) => (
            StatusCode::OK,
            axum::Json(serde_json::json!(transcription)),
        ),
        Err(e) => {
            log::error!("Failed to transcribe: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!(SafeErrorResponse::internal_error())),
            )
        }
    }
}

pub async fn generate_captions_handler(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    axum::Json(req): axum::Json<GenerateCaptionsRequest>,
) -> impl IntoResponse {
    let engine = VideoEngine::new(state.conn.clone());

    let transcription = match engine.transcribe_audio(project_id, None, None).await {
        Ok(t) => t,
        Err(e) => {
            log::error!("Transcription failed: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({ "error": SafeErrorResponse::internal_error() })),
            );
        }
    };

    let style = req.style.as_deref().unwrap_or("default");
    let max_chars = req.max_chars_per_line.unwrap_or(40);
    let font_size = req.font_size.unwrap_or(32);
    let color = req.color.as_deref().unwrap_or("#FFFFFF");
    let with_bg = req.background.is_some();

    match engine
        .generate_captions_from_transcription(CaptionRequest {
            project_id,
            transcription: &transcription,
            style,
            max_chars,
            font_size,
            color,
            with_background: with_bg,
        })
        .await
    {
        Ok(layers) => (
            StatusCode::OK,
            axum::Json(serde_json::json!({
                "captions_count": layers.len(),
                "layers": layers,
            })),
        ),
        Err(e) => {
            log::error!("Failed to generate captions: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!(SafeErrorResponse::internal_error())),
            )
        }
    }
}

pub async fn detect_scenes_handler(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
) -> impl IntoResponse {
    let engine = VideoEngine::new(state.conn.clone());
    let output_dir =
        std::env::var("VIDEO_THUMBNAILS_DIR").unwrap_or_else(|_| "./thumbnails/video".to_string());

    if let Err(e) = std::fs::create_dir_all(&output_dir) {
        log::error!("Failed to create thumbnails directory: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "error": SafeErrorResponse::internal_error() })),
        );
    }

    match engine.detect_scenes(project_id, 0.3, &output_dir).await {
        Ok(response) => (StatusCode::OK, axum::Json(serde_json::json!(response))),
        Err(e) => {
            log::error!("Scene detection failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!(SafeErrorResponse::internal_error())),
            )
        }
    }
}

pub async fn auto_reframe_handler(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    axum::Json(req): axum::Json<AutoReframeRequest>,
) -> impl IntoResponse {
    let engine = VideoEngine::new(state.conn.clone());
    let clips = match engine.get_clips(project_id).await {
        Ok(c) => c,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({ "error": "Project not found" })),
            )
        }
    };

    let clip = match clips.first() {
        Some(c) => c,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({ "error": "No clips in project" })),
            )
        }
    };

    let output_dir =
        std::env::var("VIDEO_REFRAME_DIR").unwrap_or_else(|_| "./reframed/video".to_string());
    if let Err(e) = std::fs::create_dir_all(&output_dir) {
        log::error!("Failed to create reframe directory: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "error": SafeErrorResponse::internal_error() })),
        );
    }

    match engine
        .auto_reframe(project_id, clip.id, req.target_width, req.target_height, &output_dir)
        .await
    {
        Ok(url) => (
            StatusCode::OK,
            axum::Json(serde_json::json!({ "reframed_url": url })),
        ),
        Err(e) => {
            log::error!("Auto-reframe failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!(SafeErrorResponse::internal_error())),
            )
        }
    }
}

pub async fn remove_background_handler(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    axum::Json(req): axum::Json<BackgroundRemovalRequest>,
) -> impl IntoResponse {
    let engine = VideoEngine::new(state.conn.clone());
    match engine
        .remove_background(project_id, req.clip_id, req.replacement)
        .await
    {
        Ok(response) => (StatusCode::OK, axum::Json(serde_json::json!(response))),
        Err(e) => {
            log::error!("Background removal failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!(SafeErrorResponse::internal_error())),
            )
        }
    }
}

pub async fn enhance_video_handler(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    axum::Json(req): axum::Json<VideoEnhanceRequest>,
) -> impl IntoResponse {
    let engine = VideoEngine::new(state.conn.clone());
    match engine.enhance_video(project_id, req).await {
        Ok(response) => (StatusCode::OK, axum::Json(serde_json::json!(response))),
        Err(e) => {
            log::error!("Video enhancement failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!(SafeErrorResponse::internal_error())),
            )
        }
    }
}

pub async fn beat_sync_handler(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    axum::Json(req): axum::Json<BeatSyncRequest>,
) -> impl IntoResponse {
    let engine = VideoEngine::new(state.conn.clone());
    match engine
        .detect_beats(project_id, req.audio_track_id, req.sensitivity)
        .await
    {
        Ok(response) => (StatusCode::OK, axum::Json(serde_json::json!(response))),
        Err(e) => {
            log::error!("Beat sync failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!(SafeErrorResponse::internal_error())),
            )
        }
    }
}

pub async fn generate_waveform_handler(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    axum::Json(req): axum::Json<WaveformRequest>,
) -> impl IntoResponse {
    let engine = VideoEngine::new(state.conn.clone());
    match engine
        .generate_waveform(project_id, req.audio_track_id, req.samples_per_second)
        .await
    {
        Ok(response) => (StatusCode::OK, axum::Json(serde_json::json!(response))),
        Err(e) => {
            log::error!("Waveform generation failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!(SafeErrorResponse::internal_error())),
            )
        }
    }
}
