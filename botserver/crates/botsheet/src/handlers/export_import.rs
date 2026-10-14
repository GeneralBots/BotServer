//! Export and import handlers (split from [`super::crud`]).

use base64::Engine;
use crate::auth::{resolve_user_id, SheetUser};
use crate::export::{
    export_to_csv, export_to_html, export_to_json, export_to_markdown, export_to_ods,
    export_to_pdf_data, export_to_xlsx,
};
use crate::state::{save_sheet_to_drive, SheetState};
use crate::storage::import::import_spreadsheet_bytes;
use crate::types::{ExportRequest, Spreadsheet};
use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

pub async fn handle_export_sheet(
    State(state): State<Arc<SheetState>>,
    user: Option<Extension<SheetUser>>,
    Json(req): Json<ExportRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let user_id = resolve_user_id(user.as_deref());

    let session = state
        .sessions
        .get_or_load(&state, &user_id, &req.id)
        .await
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e })),
            )
        })?;
    let sheet = session.sheet.read().await.clone();

    match req.format.as_str() {
        "csv" => {
            let csv = export_to_csv(&sheet);
            Ok(([(axum::http::header::CONTENT_TYPE, "text/csv")], csv))
        }
        "xlsx" => {
            let xlsx = export_to_xlsx(&sheet).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botsheet::handlers::export_import", None)})),
                )
            })?;
            Ok((
                [(
                    axum::http::header::CONTENT_TYPE,
                    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                )],
                xlsx,
            ))
        }
        "json" => {
            let json = export_to_json(&sheet);
            Ok(([(axum::http::header::CONTENT_TYPE, "application/json")], json))
        }
        "html" => {
            let html = export_to_html(&sheet);
            Ok(([(axum::http::header::CONTENT_TYPE, "text/html")], html))
        }
        "ods" => {
            let ods = export_to_ods(&sheet).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botsheet::handlers::export_import", None)})),
                )
            })?;
            Ok((
                [(
                    axum::http::header::CONTENT_TYPE,
                    "application/vnd.oasis.opendocument.spreadsheet",
                )],
                ods,
            ))
        }
        "md" | "markdown" => {
            let md = export_to_markdown(&sheet);
            Ok(([(axum::http::header::CONTENT_TYPE, "text/markdown")], md))
        }
        "pdf" => {
            let pdf = export_to_pdf_data(&sheet).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botsheet::handlers::export_import", None)})),
                )
            })?;
            let encoded = base64::engine::general_purpose::STANDARD.encode(&pdf);
            Ok((
                [(axum::http::header::CONTENT_TYPE, "application/pdf")],
                encoded,
            ))
        }
        _ => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Unsupported format" })),
        )),
    }
}

pub async fn handle_import_sheet(
    State(state): State<Arc<SheetState>>,
    user: Option<Extension<SheetUser>>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<Spreadsheet>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = resolve_user_id(user.as_deref());
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut filename = "import.xlsx".to_string();

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            filename = field.file_name().unwrap_or("import.xlsx").to_string();
            if let Ok(bytes) = field.bytes().await {
                file_bytes = Some(bytes.to_vec());
            }
        }
    }

    let bytes = file_bytes.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "No file uploaded" })),
        )
    })?;

    let mut sheet = import_spreadsheet_bytes(&bytes, &filename, &user_id).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        )
    })?;

    let user_id = resolve_user_id(user.as_deref());
    sheet.owner_id = user_id.clone();

    if let Err(e) = save_sheet_to_drive(&state, &user_id, &sheet).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botsheet::handlers::export_import", None)})),
        ));
    }

    Ok(Json(sheet))
}
