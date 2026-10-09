pub(crate) use chrono::{DateTime, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use uuid::Uuid;
pub(crate) use botsecurity::path_guard::sanitize_filename;
pub(crate) use botcore::shared::utils::parse_hex_color;

pub struct PdfDocument {
    name: String,
    pages: Vec<PdfPage>,
    fill_color: String,
    stroke_color: String,
}

struct PdfPage {}

#[derive(Debug, Clone)]
pub struct ExportBounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone)]
pub enum ExportError {
    InvalidFormat(String),
    RenderError(String),
    RenderFailed(String),
    IoError(String),
    EmptyCanvas,
    InvalidDimensions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Png,
    Pdf,
    Svg,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOptions {
    pub format: ExportFormat,
    pub width: u32,
    pub height: u32,
    pub background_color: Option<String>,
    pub include_grid: bool,
    pub scale: f32,
    pub padding: u32,
    pub quality: u8,
    pub include_metadata: bool,
    pub selected_shapes_only: bool,
    pub selected_shape_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub id: Uuid,
    pub whiteboard_id: Uuid,
    pub format: ExportFormat,
    pub file_name: String,
    pub file_size: u64,
    pub content_type: String,
    pub data: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub metadata: ExportMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportMetadata {
    pub whiteboard_name: String,
    pub shape_count: u32,
    pub export_dimensions: (u32, u32),
    pub original_dimensions: (u32, u32),
    pub exported_by: String,
    pub export_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhiteboardShape {
    pub id: Uuid,
    pub shape_type: ShapeType,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub rotation: f64,
    pub fill_color: Option<String>,
    pub stroke_color: Option<String>,
    pub stroke_width: f32,
    pub opacity: f32,
    pub points: Vec<Point>,
    pub text: Option<String>,
    pub font_size: Option<f32>,
    pub font_family: Option<String>,
    pub z_index: i32,
    pub locked: bool,
    pub image_data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShapeType {
    Rectangle,
    Ellipse,
    Line,
    Arrow,
    Freehand,
    Text,
    Image,
    Sticky,
    Connector,
    Triangle,
    Diamond,
    Star,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhiteboardData {
    pub id: Uuid,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub background_color: String,
    pub shapes: Vec<WhiteboardShape>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct WhiteboardExportService {
    export_history: Arc<RwLock<HashMap<Uuid, Vec<ExportResult>>>>,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;
