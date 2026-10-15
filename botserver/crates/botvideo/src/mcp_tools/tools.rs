//! MCP tool implementations for botvideo. Each function accepts a
//! deserialized input payload and returns a [`McpToolResponse`] envelope.

use std::sync::Arc;

use tracing::info;
use uuid::Uuid;

use crate::engine::CaptionRequest;
use crate::engine::VideoEngine;
use crate::requests::*;
use crate::routes::DbPool;

use super::types::*;

pub async fn create_video_project_tool(
    db: DbPool,
    input: CreateVideoProjectInput,
) -> McpToolResponse<CreateProjectOutput> {
    let engine = VideoEngine::new(db);
    let req = CreateProjectRequest {
        name: input.name.clone(),
        description: input.description,
        resolution_width: input.resolution_width,
        resolution_height: input.resolution_height,
        fps: input.fps,
    };

    match engine.create_project(None, None, req).await {
        Ok(project) => {
            info!("MCP: Created video project {} ({})", project.name, project.id);
            McpToolResponse::ok(CreateProjectOutput {
                project_id: project.id.to_string(),
                name: project.name,
                resolution: format!("{}x{}", project.resolution_width, project.resolution_height),
                fps: project.fps,
            })
        }
        Err(e) => {
            log::error!("MCP: Failed to create video project: {e}");
            McpToolResponse::err(format!("Failed to create project: {e}"))
        }
    }
}

pub async fn add_video_clip_tool(
    db: DbPool,
    input: AddVideoClipInput,
) -> McpToolResponse<AddClipOutput> {
    let project_id = match Uuid::parse_str(&input.project_id) {
        Ok(id) => id,
        Err(_) => return McpToolResponse::err("Invalid project_id format"),
    };

    let engine = VideoEngine::new(db);
    let req = AddClipRequest {
        name: input.name,
        source_url: input.source_url,
        at_ms: input.at_ms,
        duration_ms: input.duration_ms,
    };

    match engine.add_clip(project_id, req).await {
        Ok(clip) => {
            info!("MCP: Added clip {} to project {}", clip.id, project_id);
            McpToolResponse::ok(AddClipOutput {
                clip_id: clip.id.to_string(),
                project_id: clip.project_id.to_string(),
                name: clip.name,
                start_ms: clip.start_ms,
                duration_ms: clip.duration_ms,
            })
        }
        Err(e) => {
            log::error!("MCP: Failed to add clip: {e}");
            McpToolResponse::err(format!("Failed to add clip: {e}"))
        }
    }
}

pub async fn generate_captions_tool(
    db: DbPool,
    input: GenerateCaptionsInput,
) -> McpToolResponse<GenerateCaptionsOutput> {
    let project_id = match Uuid::parse_str(&input.project_id) {
        Ok(id) => id,
        Err(_) => return McpToolResponse::err("Invalid project_id format"),
    };

    let engine = VideoEngine::new(db);

    let transcription = match engine
        .transcribe_audio(project_id, None, input.language.clone())
        .await
    {
        Ok(t) => t,
        Err(e) => {
            log::error!("MCP: Transcription failed: {e}");
            return McpToolResponse::err(format!("Transcription failed: {e}"));
        }
    };

    let style = input.style.as_deref().unwrap_or("default");
    let max_chars = input.max_chars_per_line.unwrap_or(40);
    let font_size = input.font_size.unwrap_or(32);
    let color = input.color.as_deref().unwrap_or("#FFFFFF");
    let with_bg = input.with_background.unwrap_or(true);

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
        Ok(layers) => {
            info!("MCP: Generated {} captions for project {}", layers.len(), project_id);
            McpToolResponse::ok(GenerateCaptionsOutput {
                project_id: project_id.to_string(),
                captions_count: layers.len(),
                total_duration_ms: transcription.duration_ms,
                language: transcription.language,
            })
        }
        Err(e) => {
            log::error!("MCP: Failed to generate captions: {e}");
            McpToolResponse::err(format!("Failed to generate captions: {e}"))
        }
    }
}

pub async fn export_video_tool(
    db: DbPool,
    cache: Option<Arc<redis::Client>>,
    input: ExportVideoInput,
) -> McpToolResponse<ExportVideoOutput> {
    let project_id = match Uuid::parse_str(&input.project_id) {
        Ok(id) => id,
        Err(_) => return McpToolResponse::err("Invalid project_id format"),
    };

    let engine = VideoEngine::new(db);
    let format = input.format.clone().unwrap_or_else(|| "mp4".to_string());
    let quality = input.quality.clone().unwrap_or_else(|| "high".to_string());

    let req = ExportRequest {
        format: Some(format.clone()),
        quality: Some(quality.clone()),
        save_to_library: input.save_to_library,
    };

    match engine.start_export(project_id, req, cache.as_ref()).await {
        Ok(export) => {
            info!("MCP: Started export {} for project {}", export.id, project_id);
            McpToolResponse::ok(ExportVideoOutput {
                export_id: export.id.to_string(),
                project_id: export.project_id.to_string(),
                status: export.status,
                format,
                quality,
            })
        }
        Err(e) => {
            log::error!("MCP: Failed to start export: {e}");
            McpToolResponse::err(format!("Failed to start export: {e}"))
        }
    }
}

pub async fn add_text_overlay_tool(
    db: DbPool,
    input: AddTextOverlayInput,
) -> McpToolResponse<AddTextOverlayOutput> {
    let project_id = match Uuid::parse_str(&input.project_id) {
        Ok(id) => id,
        Err(_) => return McpToolResponse::err("Invalid project_id format"),
    };

    let engine = VideoEngine::new(db);
    let start_ms = input.at_ms.unwrap_or(0);
    let duration_ms = input.duration_ms.unwrap_or(5000);
    let end_ms = start_ms + duration_ms;

    let req = AddLayerRequest {
        name: Some("Text".to_string()),
        layer_type: "text".to_string(),
        start_ms: Some(start_ms),
        end_ms: Some(end_ms),
        x: input.x.or(Some(0.5)),
        y: input.y.or(Some(0.9)),
        width: Some(0.8),
        height: Some(0.1),
        properties: Some(serde_json::json!({
            "content": input.content,
            "font_family": "Arial",
            "font_size": input.font_size.unwrap_or(48),
            "color": input.color.unwrap_or_else(|| "#FFFFFF".to_string()),
            "text_align": "center",
        })),
    };

    match engine.add_layer(project_id, req).await {
        Ok(layer) => {
            info!("MCP: Added text overlay {} to project {}", layer.id, project_id);
            McpToolResponse::ok(AddTextOverlayOutput {
                layer_id: layer.id.to_string(),
                project_id: layer.project_id.to_string(),
                content: input.content,
                start_ms: layer.start_ms,
                end_ms: layer.end_ms,
            })
        }
        Err(e) => {
            log::error!("MCP: Failed to add text overlay: {e}");
            McpToolResponse::err(format!("Failed to add text overlay: {e}"))
        }
    }
}

pub async fn add_audio_track_tool(
    db: DbPool,
    input: AddAudioTrackInput,
) -> McpToolResponse<AddAudioTrackOutput> {
    let project_id = match Uuid::parse_str(&input.project_id) {
        Ok(id) => id,
        Err(_) => return McpToolResponse::err("Invalid project_id format"),
    };

    let engine = VideoEngine::new(db);
    let track_type = input.track_type.clone().unwrap_or_else(|| "music".to_string());

    let req = AddAudioRequest {
        name: input.name,
        source_url: input.source_url,
        track_type: Some(track_type.clone()),
        start_ms: input.start_ms,
        duration_ms: None,
        volume: input.volume,
    };

    match engine.add_audio_track(project_id, req).await {
        Ok(track) => {
            info!("MCP: Added audio track {} to project {}", track.id, project_id);
            McpToolResponse::ok(AddAudioTrackOutput {
                track_id: track.id.to_string(),
                project_id: track.project_id.to_string(),
                name: track.name,
                track_type,
            })
        }
        Err(e) => {
            log::error!("MCP: Failed to add audio track: {e}");
            McpToolResponse::err(format!("Failed to add audio track: {e}"))
        }
    }
}
