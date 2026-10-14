use crate::adapter::InstagramAdapter;
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
    let recipient = request.get("to").and_then(|v| v.as_str()).unwrap_or("");
    let message = request
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if recipient.is_empty() || message.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"success": false, "error": "to and message required"})),
        );
    }

    let adapter = InstagramAdapter::with_config(&state.get_config, "");
    let response = BotResponse::new(
        String::new(),
        String::new(),
        recipient.to_string(),
        message.to_string(),
        "instagram",
    );

    match adapter.send_message(response).await {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({"success": true}))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"success": false, "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e.to_string(), "botinstagram::handlers", None)})),
        ),
    }
}
