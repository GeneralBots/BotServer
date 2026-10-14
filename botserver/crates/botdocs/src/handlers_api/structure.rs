use crate::state::DocState;
use crate::storage::{get_current_user_id, load_document_from_drive};
use crate::types::{GetOutlineRequest, OutlineItem, OutlineResponse};
use crate::utils::strip_html;
use axum::{extract::State, http::StatusCode, Json};
use std::sync::Arc;

pub async fn handle_get_outline(
    State(state): State<Arc<DocState>>,
    Json(req): Json<GetOutlineRequest>,
) -> Result<Json<OutlineResponse>, (StatusCode, Json<serde_json::Value>)> {
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
                Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::structure", None)})),
            ))
        }
    };

    let mut items = Vec::new();
    let content = &doc.content;

    for level in 1..=6u32 {
        let tag = format!("<h{level}>");
        let end_tag = format!("</h{level}>");
        let mut search_pos = 0;

        while let Some(start) = content[search_pos..].find(&tag) {
            let abs_start = search_pos + start;
            if let Some(end) = content[abs_start..].find(&end_tag) {
                let text_start = abs_start + tag.len();
                let text_end = abs_start + end;
                let text = strip_html(&content[text_start..text_end]);
                let length = text_end - text_start;

                items.push(OutlineItem {
                    id: uuid::Uuid::new_v4().to_string(),
                    text,
                    level,
                    position: abs_start,
                    length,
                    style_name: format!("Heading {level}"),
                });
                search_pos = text_end + end_tag.len();
            } else {
                break;
            }
        }
    }

    items.sort_by_key(|i| i.position);

    Ok(Json(OutlineResponse { items }))
}
