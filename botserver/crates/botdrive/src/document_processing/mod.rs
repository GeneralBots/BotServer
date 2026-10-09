pub(crate) use axum::{extract::State, http::StatusCode, response::Json};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::sync::Arc;
pub(crate) use botcore::shared::state::AppState;

#[derive(Debug, Deserialize)]
pub struct MergeDocumentsRequest {
    pub bucket: String,
    pub source_paths: Vec<String>,
    pub output_path: String,
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConvertDocumentRequest {
    pub bucket: String,
    pub source_path: String,
    pub output_path: String,
    pub from_format: String,
    pub to_format: String,
}

#[derive(Debug, Deserialize)]
pub struct FillDocumentRequest {
    pub bucket: String,
    pub template_path: String,
    pub output_path: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct ExportDocumentRequest {
    pub bucket: String,
    pub source_path: String,
    pub format: String,
}

#[derive(Debug, Deserialize)]
pub struct ImportDocumentRequest {
    pub bucket: String,
    pub source_url: Option<String>,
    pub source_data: Option<String>,
    pub output_path: String,
    pub format: String,
}

#[derive(Debug, Serialize)]
pub struct DocumentResponse {
    pub success: bool,
    pub output_path: Option<String>,
    pub message: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

mod part1;
mod part2;

pub use part1::*;
pub use part2::*;
