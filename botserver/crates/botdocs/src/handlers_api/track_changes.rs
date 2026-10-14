use crate::state::DocState;
use crate::storage::{get_current_user_id, load_document_from_drive, save_document};
use crate::types::{
    AcceptRejectAllRequest, AcceptRejectChangeRequest, EnableTrackChangesRequest,
    ListTrackChangesResponse,
};
use axum::{extract::{Query, State}, http::StatusCode, Json};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;

pub async fn handle_enable_track_changes(
    State(state): State<Arc<DocState>>,
    Json(req): Json<EnableTrackChangesRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut doc = match load_document_from_drive(&state, &user_id, &req.doc_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Document not found" })))),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::track_changes", None)})))),
    };

    doc.track_changes_enabled = req.enabled;
    doc.updated_at = Utc::now();

    if let Err(e) = save_document(&state, &user_id, &doc).await {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::track_changes", None)}))));
    }

    Ok(Json(serde_json::json!({ "success": true, "enabled": req.enabled })))
}

pub async fn handle_accept_reject_change(
    State(state): State<Arc<DocState>>,
    Json(req): Json<AcceptRejectChangeRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut doc = match load_document_from_drive(&state, &user_id, &req.doc_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Document not found" })))),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::track_changes", None)})))),
    };

    if let Some(changes) = &mut doc.track_changes {
        for change in changes.iter_mut() {
            if change.id == req.change_id {
                change.accepted = Some(req.accept);
                break;
            }
        }
    }

    doc.updated_at = Utc::now();
    if let Err(e) = save_document(&state, &user_id, &doc).await {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::track_changes", None)}))));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn handle_accept_reject_all(
    State(state): State<Arc<DocState>>,
    Json(req): Json<AcceptRejectAllRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut doc = match load_document_from_drive(&state, &user_id, &req.doc_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Document not found" })))),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::track_changes", None)})))),
    };

    if let Some(changes) = &mut doc.track_changes {
        for change in changes.iter_mut() {
            change.accepted = Some(req.accept);
        }
    }

    doc.updated_at = Utc::now();
    if let Err(e) = save_document(&state, &user_id, &doc).await {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::track_changes", None)}))));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn handle_list_track_changes(
    State(state): State<Arc<DocState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ListTrackChangesResponse>, (StatusCode, Json<serde_json::Value>)> {
    let doc_id = params.get("doc_id").cloned().unwrap_or_default();
    let user_id = get_current_user_id();
    let doc = match load_document_from_drive(&state, &user_id, &doc_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Document not found" })))),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::track_changes", None)})))),
    };

    let changes = doc.track_changes.unwrap_or_default();
    Ok(Json(ListTrackChangesResponse {
        changes,
        enabled: doc.track_changes_enabled,
    }))
}
