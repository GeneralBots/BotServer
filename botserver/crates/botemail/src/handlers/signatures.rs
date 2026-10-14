use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use diesel::prelude::*;
use diesel::sql_types::{Bool, Text, Varchar};
use diesel::sql_types::Uuid as DieselUuid;
use log::warn;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::{AppState, EmailSignatureRow, extract_user_from_session};
use crate::types::{CreateSignatureRequest, UpdateSignatureRequest};

fn strip_html_tags(html: &str) -> String {
    let text = html
        .replace("&nbsp;", " ").replace("&amp;", "&")
        .replace("&lt;", "<").replace("&gt;", ">")
        .replace("&quot;", "\"").replace("&#39;", "'");

    let text = text.replace("<br>", "\n").replace("<br/>", "\n")
        .replace("<br />", "\n").replace("</p>", "\n")
        .replace("</div>", "\n").replace("</li>", "\n");

    let mut result = String::with_capacity(text.len());
    let mut in_tag = false;
    for c in text.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }

    let mut cleaned = String::new();
    let mut prev_newline = false;
    for c in result.chars() {
        if c == '\n' {
            if !prev_newline { cleaned.push(c); }
            prev_newline = true;
        } else {
            cleaned.push(c);
            prev_newline = false;
        }
    }
    cleaned.trim().to_string()
}

pub async fn list_signatures(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let user_id = match extract_user_from_session(&headers) {
        Ok(id) => id,
        Err(_) => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "error": "Authentication required", "signatures": [] }))).into_response(),
    };

    let mut conn = match state.pool.get() {
        Ok(c) => c,
        Err(e) => {
            return Json(serde_json::json!({
                "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botemail::handlers::signatures", None),
                "signatures": []
            })).into_response();
        }
    };

    let result: Result<Vec<EmailSignatureRow>, _> = diesel::sql_query(
        "SELECT id, user_id, bot_id, name, content_html, content_plain, is_default, is_active, created_at, updated_at
        FROM email_signatures WHERE user_id = $1 AND is_active = true ORDER BY is_default DESC, name ASC"
    )
    .bind::<DieselUuid, _>(user_id)
    .load(&mut conn);

    match result {
        Ok(signatures) => Json(serde_json::json!({ "signatures": signatures })).into_response(),
        Err(e) => {
            warn!("Failed to list signatures: {}", e);
            Json(serde_json::json!({
                "signatures": [{ "id": "default", "name": "Default Signature",
                "content_html": "<p>Best regards,<br>The Team</p>",
                "content_plain": "Best regards,\nThe Team", "is_default": true }]
            })).into_response()
        }
    }
}

pub async fn get_default_signature(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let user_id = match extract_user_from_session(&headers) {
        Ok(id) => id,
        Err(_) => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "error": "Authentication required" }))).into_response(),
    };

    let mut conn = match state.pool.get() {
        Ok(c) => c,
        Err(e) => {
            return Json(serde_json::json!({
                "id": "default", "name": "Default Signature",
                "content_html": "<p>Best regards,<br>The Team</p>",
                "content_plain": "Best regards,\nThe Team", "is_default": true, "_error": format!("Database connection error: {}", e)
            })).into_response();
        }
    };

    let result: Result<EmailSignatureRow, _> = diesel::sql_query(
        "SELECT id, user_id, bot_id, name, content_html, content_plain, is_default, is_active, created_at, updated_at
        FROM email_signatures WHERE user_id = $1 AND is_default = true AND is_active = true LIMIT 1"
    )
    .bind::<DieselUuid, _>(user_id)
    .get_result(&mut conn);

    match result {
        Ok(signature) => Json(serde_json::json!({
            "id": signature.id, "name": signature.name,
            "content_html": signature.content_html, "content_plain": signature.content_plain,
            "is_default": signature.is_default
        })).into_response(),
        Err(_) => Json(serde_json::json!({
            "id": "default", "name": "Default Signature",
            "content_html": "<p>Best regards,<br>The Team</p>",
            "content_plain": "Best regards,\nThe Team", "is_default": true
        })).into_response()
    }
}

pub async fn get_signature(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let user_id = match extract_user_from_session(&headers) {
        Ok(id) => id,
        Err(_) => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "error": "Authentication required" }))).into_response(),
    };

    let signature_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "Invalid signature ID" }))).into_response(),
    };

    let mut conn = match state.pool.get() {
        Ok(c) => c,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botemail::handlers::signatures", None) }))).into_response(),
    };

    let result: Result<EmailSignatureRow, _> = diesel::sql_query(
        "SELECT id, user_id, bot_id, name, content_html, content_plain, is_default, is_active, created_at, updated_at
        FROM email_signatures WHERE id = $1 AND user_id = $2"
    )
    .bind::<DieselUuid, _>(signature_id)
    .bind::<DieselUuid, _>(user_id)
    .get_result(&mut conn);

    match result {
        Ok(signature) => Json(serde_json::json!(signature)).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Signature not found" }))).into_response()
    }
}

pub async fn create_signature(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<CreateSignatureRequest>,
) -> impl IntoResponse {
    let user_id = match extract_user_from_session(&headers) {
        Ok(id) => id,
        Err(_) => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "success": false, "error": "Authentication required" }))).into_response(),
    };

    let mut conn = match state.pool.get() {
        Ok(c) => c,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "success": false, "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botemail::handlers::signatures", None) }))).into_response(),
    };

    let new_id = Uuid::new_v4();
    let content_plain = payload.content_plain.unwrap_or_else(|| strip_html_tags(&payload.content_html));

    if payload.is_default {
        let _ = diesel::sql_query("UPDATE email_signatures SET is_default = false WHERE user_id = $1 AND is_default = true")
            .bind::<DieselUuid, _>(user_id)
            .execute(&mut conn);
    }

    let result = diesel::sql_query(
        "INSERT INTO email_signatures (id, user_id, name, content_html, content_plain, is_default, is_active, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, true, NOW(), NOW()) RETURNING id"
    )
    .bind::<DieselUuid, _>(new_id).bind::<DieselUuid, _>(user_id)
    .bind::<Varchar, _>(&payload.name).bind::<Text, _>(&payload.content_html)
    .bind::<Text, _>(&content_plain).bind::<Bool, _>(payload.is_default)
    .execute(&mut conn);

    match result {
        Ok(_) => Json(serde_json::json!({ "success": true, "id": new_id, "name": payload.name })).into_response(),
        Err(e) => {
            warn!("Failed to create signature: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "success": false, "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botemail::handlers::signatures", None) }))).into_response()
        }
    }
}

pub async fn update_signature(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<UpdateSignatureRequest>,
) -> impl IntoResponse {
    let user_id = match extract_user_from_session(&headers) {
        Ok(id) => id,
        Err(_) => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "success": false, "error": "Authentication required" }))).into_response(),
    };

    let signature_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "success": false, "error": "Invalid signature ID" }))).into_response(),
    };

    let mut conn = match state.pool.get() {
        Ok(c) => c,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "success": false, "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botemail::handlers::signatures", None) }))).into_response(),
    };

    let mut updates = vec!["updated_at = NOW()".to_string()];
    if payload.name.is_some() { updates.push("name = $3".to_string()); }
    if payload.content_html.is_some() { updates.push("content_html = $4".to_string()); }
    if payload.content_plain.is_some() { updates.push("content_plain = $5".to_string()); }
    if let Some(is_default) = payload.is_default {
        if is_default {
            let _ = diesel::sql_query("UPDATE email_signatures SET is_default = false WHERE user_id = $1 AND is_default = true AND id != $2")
                .bind::<DieselUuid, _>(user_id).bind::<DieselUuid, _>(signature_id)
                .execute(&mut conn);
        }
        updates.push("is_default = $6".to_string());
    }
    if payload.is_active.is_some() { updates.push("is_active = $7".to_string()); }

    let result = diesel::sql_query(format!("UPDATE email_signatures SET {} WHERE id = $1 AND user_id = $2", updates.join(", ")))
        .bind::<DieselUuid, _>(signature_id).bind::<DieselUuid, _>(user_id)
        .bind::<Varchar, _>(payload.name.unwrap_or_default())
        .bind::<Text, _>(payload.content_html.unwrap_or_default())
        .bind::<Text, _>(payload.content_plain.unwrap_or_default())
        .bind::<Bool, _>(payload.is_default.unwrap_or(false))
        .bind::<Bool, _>(payload.is_active.unwrap_or(true))
        .execute(&mut conn);

    match result {
        Ok(rows) if rows > 0 => Json(serde_json::json!({ "success": true, "id": id })).into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "success": false, "error": "Signature not found" }))).into_response(),
        Err(e) => {
            warn!("Failed to update signature: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "success": false, "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botemail::handlers::signatures", None) }))).into_response()
        }
    }
}

pub async fn delete_signature(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let user_id = match extract_user_from_session(&headers) {
        Ok(id) => id,
        Err(_) => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "success": false, "error": "Authentication required" }))).into_response(),
    };

    let signature_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "success": false, "error": "Invalid signature ID" }))).into_response(),
    };

    let mut conn = match state.pool.get() {
        Ok(c) => c,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "success": false, "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botemail::handlers::signatures", None) }))).into_response(),
    };

    let result = diesel::sql_query("UPDATE email_signatures SET is_active = false, updated_at = NOW() WHERE id = $1 AND user_id = $2")
        .bind::<DieselUuid, _>(signature_id).bind::<DieselUuid, _>(user_id)
        .execute(&mut conn);

    match result {
        Ok(rows) if rows > 0 => Json(serde_json::json!({ "success": true, "id": id })).into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "success": false, "error": "Signature not found" }))).into_response(),
        Err(e) => {
            warn!("Failed to delete signature: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "success": false, "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botemail::handlers::signatures", None) }))).into_response()
        }
    }
}
