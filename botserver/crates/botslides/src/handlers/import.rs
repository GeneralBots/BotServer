use crate::storage::{get_current_user_id, save_presentation_to_drive, DriveOps};
use crate::types::Presentation;
use crate::utils::{create_default_theme, slides_from_markdown};
use crate::SlidesState;
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

pub async fn handle_import_presentation<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<Presentation>, (StatusCode, Json<serde_json::Value>)> {
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut filename = "import.pptx".to_string();

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            filename = field.file_name().unwrap_or("import.pptx").to_string();
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

    let ext = filename
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_lowercase();
    let theme = create_default_theme();

    let slides = match ext.as_str() {
        "md" | "markdown" => {
            let content = String::from_utf8_lossy(&bytes);
            slides_from_markdown(&content)
        }
        "json" => {
            let pres: Result<Presentation, _> = serde_json::from_slice(&bytes);
            match pres {
                Ok(p) => p.slides,
                Err(e) => {
                    // A parse error carries the reader's offsets and internals;
                    // the client is told which input was rejected. Detail: log.
                    log::warn!("slides import: JSON parse failed: {e}");
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({ "error": "Invalid JSON" })),
                    ))
                }
            }
        }
        "pptx" => crate::pptx::load_pptx(&bytes)
            .map_err(|e| {
                log::warn!("slides import: PPTX parse failed: {e}");
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": "Invalid PPTX" })),
                )
            })?
            .slides,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("Unsupported format: {ext}") })),
            ))
        }
    };

    let name = filename
        .rsplit('/')
        .next()
        .unwrap_or(&filename)
        .rsplit('.')
        .next_back()
        .unwrap_or(&filename)
        .to_string();

    let user_id = get_current_user_id();
    let presentation = Presentation {
        id: Uuid::new_v4().to_string(),
        name,
        owner_id: user_id.clone(),
        slides,
        theme,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    if let Some(drive) = &state.drive {
        if let Err(e) = save_presentation_to_drive(drive, &user_id, &presentation).await {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botslides::handlers::import", None)})),
            ));
        }
    }

    Ok(Json(presentation))
}
