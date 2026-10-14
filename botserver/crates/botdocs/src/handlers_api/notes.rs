use crate::state::DocState;
use crate::storage::{get_current_user_id, load_document_from_drive, save_document};
use crate::types::{
    AddEndnoteRequest, AddFootnoteRequest, DeleteEndnoteRequest, DeleteFootnoteRequest,
    Endnote, Footnote, ListEndnotesResponse, ListFootnotesResponse, UpdateEndnoteRequest,
    UpdateFootnoteRequest,
};
use axum::{extract::{Query, State}, http::StatusCode, Json};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;

pub async fn handle_add_footnote(
    State(state): State<Arc<DocState>>,
    Json(req): Json<AddFootnoteRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut doc = match load_document_from_drive(&state, &user_id, &req.doc_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Document not found" })))),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::notes", None)})))),
    };

    let footnotes = doc.footnotes.get_or_insert_with(Vec::new);
    let reference_mark = format!("{}", footnotes.len() + 1);

    let footnote = Footnote {
        id: uuid::Uuid::new_v4().to_string(),
        reference_mark,
        content: req.content,
        position: req.position,
    };

    footnotes.push(footnote.clone());
    doc.updated_at = Utc::now();

    if let Err(e) = save_document(&state, &user_id, &doc).await {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::notes", None)}))));
    }

    Ok(Json(serde_json::json!({ "success": true, "footnote": footnote })))
}

pub async fn handle_update_footnote(
    State(state): State<Arc<DocState>>,
    Json(req): Json<UpdateFootnoteRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut doc = match load_document_from_drive(&state, &user_id, &req.doc_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Document not found" })))),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::notes", None)})))),
    };

    if let Some(footnotes) = &mut doc.footnotes {
        for footnote in footnotes.iter_mut() {
            if footnote.id == req.footnote_id {
                footnote.content = req.content.clone();
                break;
            }
        }
    }

    doc.updated_at = Utc::now();
    if let Err(e) = save_document(&state, &user_id, &doc).await {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::notes", None)}))));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn handle_delete_footnote(
    State(state): State<Arc<DocState>>,
    Json(req): Json<DeleteFootnoteRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut doc = match load_document_from_drive(&state, &user_id, &req.doc_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Document not found" })))),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::notes", None)})))),
    };

    if let Some(footnotes) = &mut doc.footnotes {
        footnotes.retain(|f| f.id != req.footnote_id);
        for (i, footnote) in footnotes.iter_mut().enumerate() {
            footnote.reference_mark = format!("{}", i + 1);
        }
    }

    doc.updated_at = Utc::now();
    if let Err(e) = save_document(&state, &user_id, &doc).await {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::notes", None)}))));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn handle_list_footnotes(
    State(state): State<Arc<DocState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ListFootnotesResponse>, (StatusCode, Json<serde_json::Value>)> {
    let doc_id = params.get("doc_id").cloned().unwrap_or_default();
    let user_id = get_current_user_id();
    let doc = match load_document_from_drive(&state, &user_id, &doc_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Document not found" })))),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::notes", None)})))),
    };

    let footnotes = doc.footnotes.unwrap_or_default();
    Ok(Json(ListFootnotesResponse { footnotes }))
}

fn to_roman_numeral(num: usize) -> String {
    let numerals = [
        (1000, "M"), (900, "CM"), (500, "D"), (400, "CD"),
        (100, "C"), (90, "XC"), (50, "L"), (40, "XL"),
        (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I"),
    ];
    let mut result = String::new();
    let mut n = num;
    for (value, numeral) in numerals {
        while n >= value {
            result.push_str(numeral);
            n -= value;
        }
    }
    result
}

pub async fn handle_add_endnote(
    State(state): State<Arc<DocState>>,
    Json(req): Json<AddEndnoteRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut doc = match load_document_from_drive(&state, &user_id, &req.doc_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Document not found" })))),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::notes", None)})))),
    };

    let endnotes = doc.endnotes.get_or_insert_with(Vec::new);
    let reference_mark = to_roman_numeral(endnotes.len() + 1);

    let endnote = Endnote {
        id: uuid::Uuid::new_v4().to_string(),
        reference_mark,
        content: req.content,
        position: req.position,
    };

    endnotes.push(endnote.clone());
    doc.updated_at = Utc::now();

    if let Err(e) = save_document(&state, &user_id, &doc).await {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::notes", None)}))));
    }

    Ok(Json(serde_json::json!({ "success": true, "endnote": endnote })))
}

pub async fn handle_update_endnote(
    State(state): State<Arc<DocState>>,
    Json(req): Json<UpdateEndnoteRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut doc = match load_document_from_drive(&state, &user_id, &req.doc_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Document not found" })))),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::notes", None)})))),
    };

    if let Some(endnotes) = &mut doc.endnotes {
        for endnote in endnotes.iter_mut() {
            if endnote.id == req.endnote_id {
                endnote.content = req.content.clone();
                break;
            }
        }
    }

    doc.updated_at = Utc::now();
    if let Err(e) = save_document(&state, &user_id, &doc).await {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::notes", None)}))));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn handle_delete_endnote(
    State(state): State<Arc<DocState>>,
    Json(req): Json<DeleteEndnoteRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut doc = match load_document_from_drive(&state, &user_id, &req.doc_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Document not found" })))),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::notes", None)})))),
    };

    if let Some(endnotes) = &mut doc.endnotes {
        endnotes.retain(|e| e.id != req.endnote_id);
        for (i, endnote) in endnotes.iter_mut().enumerate() {
            endnote.reference_mark = to_roman_numeral(i + 1);
        }
    }

    doc.updated_at = Utc::now();
    if let Err(e) = save_document(&state, &user_id, &doc).await {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::notes", None)}))));
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn handle_list_endnotes(
    State(state): State<Arc<DocState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ListEndnotesResponse>, (StatusCode, Json<serde_json::Value>)> {
    let doc_id = params.get("doc_id").cloned().unwrap_or_default();
    let user_id = get_current_user_id();
    let doc = match load_document_from_drive(&state, &user_id, &doc_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return Err((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Document not found" })))),
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdocs::notes", None)})))),
    };

    let endnotes = doc.endnotes.unwrap_or_default();
    Ok(Json(ListEndnotesResponse { endnotes }))
}
