use crate::state::DocState;
use crate::storage::{get_current_user_id, load_document_from_drive, save_document};
use crate::types::{GenerateTocRequest, TableOfContents, TocEntry, TocResponse, UpdateTocRequest};
use crate::utils::strip_html;
use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use std::sync::Arc;

pub async fn handle_generate_toc(
    State(state): State<Arc<DocState>>,
    Json(req): Json<GenerateTocRequest>,
) -> Result<Json<TocResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut doc = match load_document_from_drive(&state, &user_id, &req.doc_id).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Document not found" })),
            ))
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::toc", None)})),
            ))
        }
    };

    let mut entries = Vec::new();
    let content = &doc.content;

    for level in 1..=req.max_level {
        let tag = format!("<h{level}>");
        let end_tag = format!("</h{level}>");
        let mut search_pos = 0;

        while let Some(start) = content[search_pos..].find(&tag) {
            let abs_start = search_pos + start;
            if let Some(end) = content[abs_start..].find(&end_tag) {
                let text_start = abs_start + tag.len();
                let text_end = abs_start + end;
                let text = strip_html(&content[text_start..text_end]);

                entries.push(TocEntry {
                    id: uuid::Uuid::new_v4().to_string(),
                    text,
                    level,
                    page_number: None,
                    position: abs_start,
                });
                search_pos = text_end + end_tag.len();
            } else {
                break;
            }
        }
    }

    entries.sort_by_key(|e| e.position);

    let toc = TableOfContents {
        id: uuid::Uuid::new_v4().to_string(),
        title: "Table of Contents".to_string(),
        entries,
        max_level: req.max_level,
        show_page_numbers: req.show_page_numbers,
        use_hyperlinks: req.use_hyperlinks,
    };

    doc.toc = Some(toc.clone());
    doc.updated_at = Utc::now();

    if let Err(e) = save_document(&state, &user_id, &doc).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::toc", None)})),
        ));
    }

    Ok(Json(TocResponse { toc }))
}

pub async fn handle_update_toc(
    State(state): State<Arc<DocState>>,
    Json(req): Json<UpdateTocRequest>,
) -> Result<Json<TocResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let doc = match load_document_from_drive(&state, &user_id, &req.doc_id).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Document not found" })),
            ))
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::toc", None)})),
            ))
        }
    };

    let existing_toc = doc.toc.unwrap_or_else(|| TableOfContents {
        id: uuid::Uuid::new_v4().to_string(),
        title: "Table of Contents".to_string(),
        entries: vec![],
        max_level: 3,
        show_page_numbers: true,
        use_hyperlinks: true,
    });

    let gen_req = GenerateTocRequest {
        doc_id: req.doc_id,
        max_level: existing_toc.max_level,
        show_page_numbers: existing_toc.show_page_numbers,
        use_hyperlinks: existing_toc.use_hyperlinks,
    };

    handle_generate_toc(State(state), Json(gen_req)).await
}
