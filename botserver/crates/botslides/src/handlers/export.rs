use crate::storage::{get_current_user_id, load_presentation_by_id, DriveOps};
use crate::types::ExportRequest;
use crate::ui::{export_to_html, export_to_json, export_to_markdown, export_to_odp_content, export_to_svg};
use crate::SlidesState;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

pub async fn handle_export_presentation<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
    Json(req): Json<ExportRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();

    let drive = state.drive.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Drive not configured" })),
        )
    })?;

    let presentation = load_presentation_by_id(drive, &user_id, &req.id)
        .await
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e })),
            )
        })?;

    if req.format == "svg" && presentation.slides.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "No slides to export" })),
        ));
    }

    let (content_type, body): (&'static str, axum::body::Body) = match req.format.as_str() {
        "html" => ("text/html", axum::body::Body::from(export_to_html(&presentation))),
        "json" => ("application/json", axum::body::Body::from(export_to_json(&presentation))),
        "svg" => (
            "image/svg+xml",
            axum::body::Body::from(export_to_svg(&presentation.slides[0], 960, 540)),
        ),
        "md" | "markdown" => ("text/markdown", axum::body::Body::from(export_to_markdown(&presentation))),
        "odp" => (
            "application/vnd.oasis.opendocument.presentation",
            axum::body::Body::from(export_to_odp_content(&presentation)),
        ),
        "pptx" => {
            let bytes = crate::pptx::export_to_pptx(&presentation).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botslides::handlers::export", None)})),
                )
            })?;
            (
                "application/vnd.openxmlformats-officedocument.presentationml.presentation",
                axum::body::Body::from(bytes),
            )
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Unsupported format" })),
            ))
        }
    };

    Ok(([(axum::http::header::CONTENT_TYPE, content_type)], body))
}
