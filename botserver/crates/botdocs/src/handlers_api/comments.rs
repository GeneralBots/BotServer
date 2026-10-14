use crate::state::DocState;
use crate::storage::{get_current_user_id, load_document_from_drive, save_document};
use crate::types::{
    AddCommentRequest, CommentReply, DeleteCommentRequest, DocumentComment,
    ListCommentsResponse, ReplyCommentRequest, ResolveCommentRequest,
};
use axum::{extract::{Query, State}, http::StatusCode, Json};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;

pub async fn handle_add_comment(
    State(state): State<Arc<DocState>>,
    Json(req): Json<AddCommentRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
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
                Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::comments", None)})),
            ))
        }
    };

    let comment = DocumentComment {
        id: uuid::Uuid::new_v4().to_string(),
        author_id: user_id.clone(),
        author_name: "User".to_string(),
        content: req.content,
        position: req.position,
        length: req.length,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        replies: vec![],
        resolved: false,
    };

    let comments = doc.comments.get_or_insert_with(Vec::new);
    comments.push(comment.clone());
    doc.updated_at = Utc::now();

    if let Err(e) = save_document(&state, &user_id, &doc).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::comments", None)})),
        ));
    }

    Ok(Json(serde_json::json!({ "success": true, "comment": comment })))
}

pub async fn handle_reply_comment(
    State(state): State<Arc<DocState>>,
    Json(req): Json<ReplyCommentRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
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
                Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::comments", None)})),
            ))
        }
    };

    if let Some(comments) = &mut doc.comments {
        for comment in comments.iter_mut() {
            if comment.id == req.comment_id {
                let reply = CommentReply {
                    id: uuid::Uuid::new_v4().to_string(),
                    author_id: user_id.clone(),
                    author_name: "User".to_string(),
                    content: req.content.clone(),
                    created_at: Utc::now(),
                };
                comment.replies.push(reply);
                comment.updated_at = Utc::now();
                break;
            }
        }
    }

    doc.updated_at = Utc::now();
    if let Err(e) = save_document(&state, &user_id, &doc).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::comments", None)})),
        ));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn handle_resolve_comment(
    State(state): State<Arc<DocState>>,
    Json(req): Json<ResolveCommentRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
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
                Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::comments", None)})),
            ))
        }
    };

    if let Some(comments) = &mut doc.comments {
        for comment in comments.iter_mut() {
            if comment.id == req.comment_id {
                comment.resolved = req.resolved;
                comment.updated_at = Utc::now();
                break;
            }
        }
    }

    doc.updated_at = Utc::now();
    if let Err(e) = save_document(&state, &user_id, &doc).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::comments", None)})),
        ));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn handle_delete_comment(
    State(state): State<Arc<DocState>>,
    Json(req): Json<DeleteCommentRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
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
                Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::comments", None)})),
            ))
        }
    };

    if let Some(comments) = &mut doc.comments {
        comments.retain(|c| c.id != req.comment_id);
    }

    doc.updated_at = Utc::now();
    if let Err(e) = save_document(&state, &user_id, &doc).await {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::comments", None)})),
        ));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn handle_list_comments(
    State(state): State<Arc<DocState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ListCommentsResponse>, (StatusCode, Json<serde_json::Value>)> {
    let doc_id = params.get("doc_id").cloned().unwrap_or_default();
    let user_id = get_current_user_id();
    let doc = match load_document_from_drive(&state, &user_id, &doc_id).await {
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
                Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::comments", None)})),
            ))
        }
    };

    let comments = doc.comments.unwrap_or_default();
    Ok(Json(ListCommentsResponse { comments }))
}
