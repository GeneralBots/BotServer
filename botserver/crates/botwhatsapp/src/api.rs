use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::models::SendMessageRequest;
use crate::state::WhatsAppState;
use crate::message_processing::{process_outbound_message, send_outbound_message};
use crate::session_management::get_bot_for_phone;
use crate::utils::format_phone_number;

pub async fn handle_send_message(
    State(state): State<Arc<WhatsAppState>>,
    Json(payload): Json<SendMessageRequest>,
) -> impl IntoResponse {
    let formatted_phone = format_phone_number(&payload.to);

    let bot_id = match payload.bot_id {
        Some(ref id) => id.parse::<Uuid>().unwrap_or_default(),
        None => {
            match get_bot_for_phone(&state, &formatted_phone) {
                Ok((id, _)) => id,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botwhatsapp::api", None)})),
                    );
                }
            }
        }
    };

    let user_id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, format!("wa:{}", formatted_phone).as_bytes());
    let session_id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, format!("wa-session:{}", formatted_phone).as_bytes());

    process_outbound_message(&state, &user_id, &session_id, &payload.message);

    match send_outbound_message(&state, &bot_id, &formatted_phone, &payload.message).await {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "sent"})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botwhatsapp::api", None)})),
        ),
    }
}

pub async fn handle_status(
    State(state): State<Arc<WhatsAppState>>,
) -> impl IntoResponse {
    let verify_token = (state.secrets)("whatsapp_verify_token");
    let api_key = (state.secrets)("whatsapp_api_key");

    let configured = verify_token.is_ok() && api_key.is_ok();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": if configured { "configured" } else { "not_configured" },
            "webhook": crate::WHATSAPP_WEBHOOK,
        })),
    )
}

pub async fn handle_sessions(
    State(state): State<Arc<WhatsAppState>>,
) -> impl IntoResponse {
    use diesel::prelude::*;
    use crate::schema::user_sessions;

    let mut conn = match state.pool.get() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botwhatsapp::api", None)})),
            );
        }
    };

    match user_sessions::table
        .limit(100)
        .load::<crate::models::UserSession>(&mut conn)
    {
        Ok(sessions) => (
            StatusCode::OK,
            Json(serde_json::json!({"sessions": sessions})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botwhatsapp::api", None)})),
        ),
    }
}
