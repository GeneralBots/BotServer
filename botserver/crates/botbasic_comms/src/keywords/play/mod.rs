pub(crate) use botbasic_types::UserSession;
pub(crate) use botbasic_types::BasicRuntime;
pub(crate) use log::{info, trace};
pub(crate) use rhai::{Dynamic, Engine};
pub(crate) use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContentType {
    Video,
    Audio,
    Image,
    Presentation,
    Document,
    Code,
    Spreadsheet,
    Pdf,
    Markdown,
    Html,
    Iframe,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlayOptions {
    pub autoplay: bool,
    pub loop_content: bool,
    pub fullscreen: bool,
    pub muted: bool,
    pub controls: bool,
    pub start_time: Option<f64>,
    pub end_time: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub theme: Option<String>,
    pub line_numbers: Option<bool>,
    pub highlight_lines: Option<Vec<u32>>,
    pub slide: Option<u32>,
    pub page: Option<u32>,
    pub zoom: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayContent {
    pub id: Uuid,
    pub session_id: Uuid,
    pub content_type: ContentType,
    pub source: String,
    pub title: Option<String>,
    pub options: PlayOptions,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayResponse {
    pub player_id: Uuid,
    pub content_type: ContentType,
    pub component: String,
    pub source_url: String,
    pub title: String,
    pub options: PlayOptions,
    pub metadata: HashMap<String, String>,
}

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

mod part1;
mod part2;
mod part3;
mod part4;

pub use part3::*;
pub use part4::*;
