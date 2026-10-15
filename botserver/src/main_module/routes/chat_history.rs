use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use diesel::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use botcore::shared::state::AppState;

use super::anonymous_auth::{resolve_chat_user_uuid, resolve_session_user};

const DEFAULT_SESSION_LIMIT: i64 = 50;
const TITLE_MAX_CHARS: usize = 80;

type HandlerError = (StatusCode, String);

#[derive(Serialize)]
struct HistorySession {
    session_id: Uuid,
    bot_id: Uuid,
    title: String,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize)]
struct HistoryMessage {
    role: String,
    content: String,
}

fn bearer_user(request: &axum::extract::Request) -> Result<Uuid, HandlerError> {
    let (user_raw, _roles, is_authenticated) = resolve_session_user(request);
    if !is_authenticated || user_raw.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Authentication required".to_string(),
        ));
    }
    Ok(resolve_chat_user_uuid(&user_raw))
}

fn db_error(context: &str, e: String) -> HandlerError {
    log::error!("Chat history error ({context}): {e}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to load chat history".to_string(),
    )
}

/// Lists the authenticated user's chat conversations, newest first. An
/// optional `bot_name` narrows the listing to a single bot; `title` is the
/// decrypted first user message of the conversation.
pub async fn handle_chat_history_sessions(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
    request: axum::extract::Request,
) -> Result<Json<serde_json::Value>, HandlerError> {
    let user_uuid = bearer_user(&request)?;
    let limit = params
        .get("limit")
        .and_then(|v| v.parse::<i64>().ok())
        .map(|v| v.clamp(1, 200))
        .unwrap_or(DEFAULT_SESSION_LIMIT);
    let bot_filter = params.get("bot_name").cloned().unwrap_or_default();

    let sessions = {
        let mut sm = state.session_manager.lock().await;
        sm.get_user_sessions(user_uuid)
            .map_err(|e| db_error("list sessions", e))?
    };

    let mut conn = state.conn.get().map_err(|e| {
        log::error!("Chat history DB pool error: {e}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        )
    })?;

    let bot_id_filter: Option<Uuid> = if bot_filter.is_empty() {
        None
    } else {
        use botcoresession::schema::bots::dsl::*;
        bots.filter(name.eq(&bot_filter))
            .select(id)
            .first::<Uuid>(&mut conn)
            .optional()
            .map_err(|e| db_error("bot lookup", e.to_string()))?
    };

    let mut filtered: Vec<botlib::models::UserSession> = sessions
        .into_iter()
        .filter(|s| match bot_id_filter {
            Some(bid) => s.bot_id == bid,
            None => true,
        })
        .collect();
    filtered.sort_by_key(|a| std::cmp::Reverse(a.updated_at));
    filtered.truncate(limit as usize);

    let mut out: Vec<HistorySession> = Vec::with_capacity(filtered.len());
    for sess in filtered {
        let title = {
            let mut sm = state.session_manager.lock().await;
            match sm.get_first_user_message(sess.id) {
                Ok(Some(first)) => first,
                Ok(None) => String::new(),
                Err(e) => {
                    log::warn!("Failed to load title for session {}: {e}", sess.id);
                    String::new()
                }
            }
        };
        let trimmed = title.trim().replace('\n', " ");
        let title = if trimmed.is_empty() {
            format!("Conversation · {}", sess.created_at.format("%b %d, %H:%M"))
        } else if trimmed.chars().count() > TITLE_MAX_CHARS {
            format!("{}…", trimmed.chars().take(TITLE_MAX_CHARS).collect::<String>())
        } else {
            trimmed
        };
        out.push(HistorySession {
            session_id: sess.id,
            bot_id: sess.bot_id,
            title,
            created_at: sess.created_at.to_rfc3339(),
            updated_at: sess.updated_at.to_rfc3339(),
        });
    }

    Ok(Json(serde_json::json!({ "sessions": out })))
}

/// Returns the decrypted messages of one conversation. The session must
/// belong to the authenticated user.
pub async fn handle_chat_history_messages(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<Uuid>,
    request: axum::extract::Request,
) -> Result<Json<serde_json::Value>, HandlerError> {
    let user_uuid = bearer_user(&request)?;

    let owner = {
        let mut sm = state.session_manager.lock().await;
        sm.get_session_by_id(session_id)
            .map_err(|e| db_error("load session", e))?
    };
    let Some(sess) = owner else {
        return Err((StatusCode::NOT_FOUND, "Conversation not found".to_string()));
    };
    if sess.user_id != user_uuid {
        return Err((StatusCode::FORBIDDEN, "Not your conversation".to_string()));
    }

    let history = {
        let mut sm = state.session_manager.lock().await;
        sm.get_conversation_history(session_id, user_uuid, Some(500))
            .map_err(|e| db_error("decrypt history", e))?
    };

    let messages: Vec<HistoryMessage> = history
        .into_iter()
        .filter(|(role, content)| {
            (*role == "user" || *role == "assistant") && !content.trim().is_empty()
        })
        .map(|(role, content)| HistoryMessage { role, content })
        .collect();

    Ok(Json(serde_json::json!({
        "session_id": session_id,
        "messages": messages,
    })))
}

pub fn configure_chat_history_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/chat/history/sessions", get(handle_chat_history_sessions))
        .route(
            "/api/chat/history/sessions/:session_id/messages",
            get(handle_chat_history_messages),
        )
}
