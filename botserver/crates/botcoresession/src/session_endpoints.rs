use crate::session_manager::SessionManager;
use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

pub use botlib::db_pool::DbPool;

pub struct SessionState {
    pub conn: DbPool,
}

pub async fn create_session(Extension(state): Extension<Arc<SessionState>>) -> impl IntoResponse {
    let temp_session_id = Uuid::new_v4();
    let user_id =
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap_or_default();
    let bot_id = Uuid::nil();

    let mut sm = SessionManager::new(
        state.conn.clone(),
        #[cfg(feature = "cache")]
        None,
    );

    if let Ok(Some(session)) = sm.get_or_create_user_session(user_id, bot_id, "New Conversation") {
        return (
            StatusCode::OK,
            Json(serde_json::json!({
                "session_id": session.id,
                "title": "New Conversation",
                "created_at": Utc::now()
            })),
        );
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "session_id": temp_session_id,
            "title": "New Conversation",
            "created_at": Utc::now(),
            "temporary": true
        })),
    )
}

pub async fn get_sessions(Extension(state): Extension<Arc<SessionState>>) -> impl IntoResponse {
    let user_id =
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap_or_default();

    let mut sm = SessionManager::new(
        state.conn.clone(),
        #[cfg(feature = "cache")]
        None,
    );

    match sm.get_user_sessions(user_id) {
        Ok(sessions) => (StatusCode::OK, Json(serde_json::json!(sessions))),
        Err(_) => (StatusCode::OK, Json(serde_json::json!([]))),
    }
}

pub async fn start_session(
    Extension(state): Extension<Arc<SessionState>>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    match Uuid::parse_str(&session_id) {
        Ok(session_uuid) => {
            let mut sm = SessionManager::new(
                state.conn.clone(),
                #[cfg(feature = "cache")]
                None,
            );

            match sm.get_session_by_id(session_uuid) {
                Ok(Some(_)) => {
                    sm.mark_waiting(session_uuid);
                    (
                        StatusCode::OK,
                        Json(serde_json::json!({"status": "started", "session_id": session_id})),
                    )
                }
                Ok(None) => (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": "Session not found"})),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e.to_string(), "botcoresession::session_endpoints", None)})),
                ),
            }
        }
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid session ID"})),
        ),
    }
}

pub async fn get_session_history(
    Extension(state): Extension<Arc<SessionState>>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let user_id =
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap_or_default();

    match Uuid::parse_str(&session_id) {
        Ok(session_uuid) => {
            let mut sm = SessionManager::new(
                state.conn.clone(),
                #[cfg(feature = "cache")]
                None,
            );

            match sm.get_conversation_history(session_uuid, user_id, None) {
                Ok(history) => (StatusCode::OK, Json(serde_json::json!(history))),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e.to_string(), "botcoresession::session_endpoints", None)})),
                ),
            }
        }
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid session ID"})),
        ),
    }
}
