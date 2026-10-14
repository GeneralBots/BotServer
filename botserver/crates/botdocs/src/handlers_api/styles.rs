use crate::state::DocState;
use crate::storage::{get_current_user_id, load_document_from_drive, save_document};
use crate::types::{
    ApplyStyleRequest, CreateStyleRequest, DeleteStyleRequest, ListStylesResponse,
    UpdateStyleRequest,
};
use axum::{extract::{Query, State}, http::StatusCode, Json};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;

pub async fn handle_create_style(
    State(state): State<Arc<DocState>>,
    Json(req): Json<CreateStyleRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut doc = match load_document_from_drive(&state, &user_id, &req.doc_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Document not found" })))),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::styles", None)})))),
    };

    let styles = doc.styles.get_or_insert_with(Vec::new);
    styles.push(req.style.clone());
    doc.updated_at = Utc::now();

    if let Err(e) = save_document(&state, &user_id, &doc).await {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::styles", None)}))));
    }

    Ok(Json(serde_json::json!({ "success": true, "style": req.style })))
}

pub async fn handle_update_style(
    State(state): State<Arc<DocState>>,
    Json(req): Json<UpdateStyleRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut doc = match load_document_from_drive(&state, &user_id, &req.doc_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Document not found" })))),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::styles", None)})))),
    };

    if let Some(styles) = &mut doc.styles {
        for style in styles.iter_mut() {
            if style.id == req.style.id {
                *style = req.style.clone();
                break;
            }
        }
    }

    doc.updated_at = Utc::now();
    if let Err(e) = save_document(&state, &user_id, &doc).await {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::styles", None)}))));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn handle_delete_style(
    State(state): State<Arc<DocState>>,
    Json(req): Json<DeleteStyleRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut doc = match load_document_from_drive(&state, &user_id, &req.doc_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Document not found" })))),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::styles", None)})))),
    };

    if let Some(styles) = &mut doc.styles {
        styles.retain(|s| s.id != req.style_id);
    }

    doc.updated_at = Utc::now();
    if let Err(e) = save_document(&state, &user_id, &doc).await {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::styles", None)}))));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn handle_list_styles(
    State(state): State<Arc<DocState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ListStylesResponse>, (StatusCode, Json<serde_json::Value>)> {
    let doc_id = params.get("doc_id").cloned().unwrap_or_default();
    let user_id = get_current_user_id();
    let doc = match load_document_from_drive(&state, &user_id, &doc_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Document not found" })))),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::styles", None)})))),
    };

    let styles = doc.styles.unwrap_or_default();
    Ok(Json(ListStylesResponse { styles }))
}

pub async fn handle_apply_style(
    State(state): State<Arc<DocState>>,
    Json(req): Json<ApplyStyleRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let doc = match load_document_from_drive(&state, &user_id, &req.doc_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Document not found" })))),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::styles", None)})))),
    };

    let style = doc
        .styles
        .as_ref()
        .and_then(|styles| styles.iter().find(|s| s.id == req.style_id))
        .cloned();

    if style.is_none() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Style not found" })),
        ));
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "style": style,
        "position": req.position,
        "length": req.length
    })))
}
