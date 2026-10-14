use crate::adapter::TeamsAdapter;
use crate::channel::ChannelAdapter;
use crate::state::ChannelState;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use botlib::models::BotResponse;
use std::sync::Arc;

pub async fn send_message(
    State(state): State<Arc<ChannelState>>,
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    let conversation_id = request
        .get("conversation_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let message = request
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if conversation_id.is_empty() || message.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"success": false, "error": "conversation_id and message required"})),
        );
    }

    let bot_id = {
        let mut conn = match state.conn.get() {
            Ok(c) => c,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"success": false, "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botmsteams::handlers", None)})),
                );
            }
        };
        (state.get_default_bot)(&mut conn).0
    };

    let adapter = TeamsAdapter::new(state.conn.clone(), bot_id, state.get_config.clone());

    let response = BotResponse::new(
        bot_id.to_string(),
        conversation_id.to_string(),
        conversation_id.to_string(),
        message.to_string(),
        "teams",
    );

    match adapter.send_message(response).await {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({"success": true}))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"success": false, "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e.to_string(), "botmsteams::handlers", None)})),
        ),
    }
}
