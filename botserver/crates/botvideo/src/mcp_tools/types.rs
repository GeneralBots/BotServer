//! Data types for the botvideo MCP tools: generic response envelope plus the
//! per-tool input and output payloads.

use serde::{Deserialize, Serialize};

/// Generic envelope returned by every MCP tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> McpToolResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVideoProjectInput {
    pub name: String,
    pub description: Option<String>,
    pub resolution_width: Option<i32>,
    pub resolution_height: Option<i32>,
    pub fps: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddVideoClipInput {
    pub project_id: String,
    pub source_url: String,
    pub name: Option<String>,
    pub at_ms: Option<i64>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateCaptionsInput {
    pub project_id: String,
    pub style: Option<String>,
    pub max_chars_per_line: Option<i32>,
    pub font_size: Option<i32>,
    pub color: Option<String>,
    pub with_background: Option<bool>,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportVideoInput {
    pub project_id: String,
    pub format: Option<String>,
    pub quality: Option<String>,
    pub save_to_library: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddTextOverlayInput {
    pub project_id: String,
    pub content: String,
    pub at_ms: Option<i64>,
    pub duration_ms: Option<i64>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub font_size: Option<i32>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddAudioTrackInput {
    pub project_id: String,
    pub source_url: String,
    pub name: Option<String>,
    pub track_type: Option<String>,
    pub start_ms: Option<i64>,
    pub volume: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectOutput {
    pub project_id: String,
    pub name: String,
    pub resolution: String,
    pub fps: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddClipOutput {
    pub clip_id: String,
    pub project_id: String,
    pub name: String,
    pub start_ms: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateCaptionsOutput {
    pub project_id: String,
    pub captions_count: usize,
    pub total_duration_ms: i64,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportVideoOutput {
    pub export_id: String,
    pub project_id: String,
    pub status: String,
    pub format: String,
    pub quality: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddTextOverlayOutput {
    pub layer_id: String,
    pub project_id: String,
    pub content: String,
    pub start_ms: i64,
    pub end_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddAudioTrackOutput {
    pub track_id: String,
    pub project_id: String,
    pub name: String,
    pub track_type: String,
}
