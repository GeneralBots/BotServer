use crate::AttendanceConfig;
use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use diesel::prelude::*;
use futures::{SinkExt, StreamExt};
use log::{debug, info, warn};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use botsecurity_crypto::encryption::{encrypt_field, derive_scope_key};

pub fn configure_attendance_routes() -> Router<Arc<AttendanceConfig>> {
    let router = Router::new()
        .route("/api/attendance/queue", get(crate::queue_handlers::list_queue))
        .route("/api/attendance/attendants", get(crate::queue_handlers::list_attendants))
        .route("/api/attendance/assign", post(crate::queue_handlers::assign_conversation))
        .route("/api/attendance/assign-by-skill", post(crate::queue_handlers::assign_by_skill))
        .route("/api/attendance/transfer", post(crate::queue_handlers::transfer_conversation))
        .route("/api/attendance/resolve", post(crate::queue_handlers::resolve_conversation))
        .route("/api/attendance/insights/:session_id", get(crate::queue_handlers::get_insights))
        .route("/api/attendance/kanban", get(crate::queue_handlers::get_kanban))
        .route("/api/attendance/respond", post(attendant_respond))
        .route("/api/attendance/ws", get(attendant_websocket_handler))
        .route("/api/attendance/webhooks", get(crate::webhooks::list_webhooks).post(crate::webhooks::create_webhook))
        .route("/api/attendance/webhooks/:id", get(crate::webhooks::get_webhook).put(crate::webhooks::update_webhook).delete(crate::webhooks::delete_webhook))
        .route("/api/attendance/webhooks/:id/test", post(crate::webhooks::test_webhook));

    #[cfg(feature = "llm")]
    let router = router
        .route("/api/attendance/llm-assist/tips", post(crate::llm_assist_handlers::generate_tips))
        .route("/api/attendance/llm-assist/polish", post(crate::llm_assist_handlers::polish_message))
        .route("/api/attendance/llm-assist/replies", post(crate::llm_assist_handlers::generate_smart_replies))
        .route("/api/attendance/llm-assist/summary/:session_id", get(crate::llm_assist_handlers::generate_summary))
        .route("/api/attendance/llm-assist/sentiment", post(crate::llm_assist_handlers::analyze_sentiment))
        .route("/api/attendance/llm-assist/config/:bot_id", get(crate::llm_assist_handlers::get_llm_config));

    router
}

#[derive(Debug, Deserialize)]
pub struct AttendantRespondRequest {
    pub session_id: String,
    pub message: String,
    pub attendant_id: String,
}

#[derive(Debug, serde::Serialize)]
pub struct AttendantRespondResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub async fn attendant_respond(
    State(config): State<Arc<AttendanceConfig>>,
    Json(request): Json<AttendantRespondRequest>,
) -> impl IntoResponse {
    info!("Attendant {} responding to session {}", request.attendant_id, request.session_id);

    let Ok(session_id) = Uuid::parse_str(&request.session_id) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(AttendantRespondResponse {
                success: false,
                message: "Invalid session ID".to_string(),
                error: Some("Could not parse session ID as UUID".to_string()),
            }),
        );
    };

    let session_result = tokio::task::spawn_blocking({
        let pool = config.pool.clone();
        move || {
            let mut db_conn = pool.get().ok()?;
            use crate::schema::user_sessions;
            user_sessions::table
                .find(session_id)
                .first::<crate::models::UserSession>(&mut db_conn)
                .ok()
        }
    })
    .await
    .ok()
    .flatten();

    let Some(session) = session_result else {
        return (
            StatusCode::NOT_FOUND,
            Json(AttendantRespondResponse {
                success: false,
                message: "Session not found".to_string(),
                error: Some("No session with that ID exists".to_string()),
            }),
        );
    };

    if let Err(e) = save_attendance_message(&config, &session, &request.message, "attendant").await {
        log::error!("Failed to save attendant message: {}", e);
    }

    if let Some(ref send_response) = config.send_bot_response {
        let response = crate::models::BotResponse {
            bot_id: session.bot_id.to_string(),
            session_id: session.id.to_string(),
            user_id: session.user_id.clone(),
            channel: session.context_data.get("channel").and_then(|v| v.as_str()).unwrap_or("web").to_string(),
            content: request.message.clone(),
            ..Default::default()
        };
        if let Err(e) = send_response(response) {
            warn!("Failed to send bot response: {}", e);
        }
    }

    if let Some(ref broadcast_fn) = config.broadcast_notification {
        let notification = crate::models::AttendantNotification {
            notification_type: "attendant_response".to_string(),
            session_id: session.id.to_string(),
            user_id: session.user_id.clone(),
            user_name: session.context_data.get("name").and_then(|v| v.as_str()).map(String::from),
            user_phone: session.context_data.get("phone").and_then(|v| v.as_str()).map(String::from),
            channel: session.context_data.get("channel").and_then(|v| v.as_str()).unwrap_or("web").to_string(),
            content: request.message.clone(),
            timestamp: Utc::now().to_rfc3339(),
            assigned_to: Some(request.attendant_id.clone()),
            priority: 0,
        };
        broadcast_fn(notification);
    }

    (
        StatusCode::OK,
        Json(AttendantRespondResponse {
            success: true,
            message: "Response sent".to_string(),
            error: None,
        }),
    )
}

pub async fn save_attendance_message(
    config: &Arc<AttendanceConfig>,
    session: &crate::models::UserSession,
    content: &str,
    sender: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(ref save_fn) = config.save_message {
        save_fn(session.id, content, sender, if sender == "user" { 0 } else { 2 })?;
        return Ok(());
    }
    let pool = config.pool.clone();
    let session_id = session.id;

    let key_bytes = derive_scope_key(&config.master_key, "message", &session.bot_id);
    let encrypted_content = encrypt_field(content, &key_bytes)?;

    let sender_clone = sender.to_string();
    tokio::task::spawn_blocking(move || {
        let mut db_conn = pool.get().map_err(|e| format!("DB error: {}", e))?;
        use crate::schema::message_history;
        diesel::insert_into(message_history::table)
            .values((
                message_history::id.eq(Uuid::new_v4()),
                message_history::session_id.eq(session_id),
                message_history::user_id.eq(session_id),
                message_history::role.eq(if sender_clone == "user" { 1 } else { 2 }),
                message_history::content_encrypted.eq(encrypted_content),
                message_history::message_type.eq(1),
                message_history::message_index.eq(0i32),
                message_history::created_at.eq(diesel::dsl::now),
            ))
            .execute(&mut db_conn)
            .map_err(|e| format!("Insert error: {}", e))?;
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| format!("Task error: {}", e))??;
    Ok(())
}

pub async fn attendant_websocket_handler(
    ws: WebSocketUpgrade,
    State(config): State<Arc<AttendanceConfig>>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let attendant_id = params.get("attendant_id").cloned();
    let Some(attendant_id) = attendant_id else {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "attendant_id is required" }))).into_response();
    };
    info!("Attendant WebSocket connection request from: {}", attendant_id);
    ws.on_upgrade(move |socket| handle_attendant_websocket(socket, config, attendant_id)).into_response()
}

async fn handle_attendant_websocket(socket: WebSocket, _config: Arc<AttendanceConfig>, attendant_id: String) {
    let (mut sender, mut receiver) = socket.split();
    info!("Attendant WebSocket connected: {}", attendant_id);
    let welcome = serde_json::json!({
        "type": "connected", "attendant_id": attendant_id,
        "message": "Connected to attendant notification service", "timestamp": Utc::now().to_rfc3339()
    });
    if let Ok(welcome_str) = serde_json::to_string(&welcome) {
        if sender.send(Message::Text(welcome_str)).await.is_err() {
            log::error!("Failed to send welcome message to attendant");
            return;
        }
    }
    let attendant_id_clone = attendant_id.clone();
    let mut send_task = tokio::spawn(async move {
        if let Err(e) = sender.send(Message::Text(serde_json::json!({
            "type": "keepalive", "timestamp": Utc::now().to_rfc3339()
        }).to_string())).await {
            debug!("Keepalive failed for attendant {}: {}", attendant_id_clone, e);
        }
    });
    let attendant_id_for_recv = attendant_id.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    debug!("Received message from attendant {}: {}", attendant_id_for_recv, text);
                }
                Message::Ping(_) => {
                    debug!("Received ping from attendant {}", attendant_id_for_recv);
                }
                Message::Close(_) => {
                    info!("Attendant {} WebSocket close requested", attendant_id_for_recv);
                    break;
                }
                _ => {}
            }
        }
    });
    tokio::select! {
        _ = (&mut send_task) => { recv_task.abort(); }
        _ = (&mut recv_task) => { send_task.abort(); }
    }
    info!("Attendant WebSocket disconnected: {}", attendant_id);
}
