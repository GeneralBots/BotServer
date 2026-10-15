//! Handlers for standalone meet group conversations. All persistence is
//! delegated to the DB-backed store modules (conversation_store,
//! conversation_messages, conversation_calls).

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::Json,
};
use std::sync::Arc;
use uuid::Uuid;

use botcore::shared::state::AppState;
use botsecurity::auth::AuthenticatedUser;

pub use crate::conversation_types::*;

use crate::{
    conversation_calls, conversation_messages, conversation_store,
};

fn db_err(e: impl std::fmt::Display) -> (StatusCode, Json<serde_json::Value>) {
    log::error!("Conversation store error: {e}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": botsecurity::error_sanitizer::log_and_sanitize_json(&e.to_string(), "botmeet::conversations", None) })),
    )
}

pub async fn create_conversation(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<CreateConversationRequest>,
) -> Result<Json<ConversationResponse>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();
    let bot_id = Uuid::nil();
    let mut req = req;
    if user.user_id != Uuid::nil() && !req.participants.contains(&user.user_id) {
        req.participants.push(user.user_id);
    }
    match conversation_store::create_conversation(&pool, bot_id, &req).await {
        Ok(c) => Ok(Json(c)),
        Err(e) => Err(db_err(e)),
    }
}

pub async fn join_conversation(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(conversation_id): Path<Uuid>,
    Json(req): Json<JoinConversationRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();
    let mut req = req;
    req.user_id = user.user_id;
    match conversation_store::join_conversation(&pool, conversation_id, &req).await {
        Ok(r) => Ok(Json(r)),
        Err(e) => Err(db_err(e)),
    }
}

pub async fn leave_conversation(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(conversation_id): Path<Uuid>,
    Json(req): Json<LeaveConversationRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();
    let mut req = req;
    req.user_id = user.user_id;
    match conversation_store::leave_conversation(&pool, conversation_id, &req).await {
        Ok(r) => Ok(Json(r)),
        Err(e) => Err(db_err(e)),
    }
}

pub async fn get_conversation_members(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<Uuid>,
) -> Result<Json<Vec<ParticipantResponse>>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();
    match conversation_store::get_conversation_members(&pool, conversation_id).await {
        Ok(members) => Ok(Json(members)),
        Err(e) => Err(db_err(e)),
    }
}

pub async fn get_conversation_messages(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<Uuid>,
) -> Result<Json<Vec<MessageResponse>>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();
    match conversation_messages::get_conversation_messages(&pool, conversation_id).await {
        Ok(messages) => Ok(Json(messages)),
        Err(e) => Err(db_err(e)),
    }
}

pub async fn send_message(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(conversation_id): Path<Uuid>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<MessageResponse>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();
    let sender_id = if user.user_id == Uuid::nil() { None } else { Some(user.user_id) };
    let sender_name = if user.username.is_empty() { None } else { Some(user.username.as_str()) };
    match conversation_messages::send_message(&pool, conversation_id, &req, sender_id, sender_name).await {
        Ok(m) => Ok(Json(m)),
        Err(e) => Err(db_err(e)),
    }
}

pub async fn edit_message(
    State(state): State<Arc<AppState>>,
    Path((conversation_id, message_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<EditMessageRequest>,
) -> Result<Json<MessageResponse>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();
    match conversation_messages::edit_message(&pool, conversation_id, message_id, &req).await {
        Ok(m) => Ok(Json(m)),
        Err(e) => Err(db_err(e)),
    }
}

pub async fn delete_message(
    State(state): State<Arc<AppState>>,
    Path((conversation_id, message_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();
    match conversation_messages::delete_message(&pool, conversation_id, message_id).await {
        Ok(r) => Ok(Json(r)),
        Err(e) => Err(db_err(e)),
    }
}

pub async fn react_to_message(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path((conversation_id, message_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<ReactToMessageRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();
    let user_id = if user.user_id == Uuid::nil() { Uuid::new_v4() } else { user.user_id };
    match conversation_messages::react_to_message(&pool, conversation_id, message_id, user_id, &req.reaction).await {
        Ok(r) => Ok(Json(r)),
        Err(e) => Err(db_err(e)),
    }
}

pub async fn pin_message(
    State(state): State<Arc<AppState>>,
    Path((conversation_id, message_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();
    match conversation_messages::pin_message(&pool, conversation_id, message_id).await {
        Ok(r) => Ok(Json(r)),
        Err(e) => Err(db_err(e)),
    }
}

pub async fn search_messages(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<Uuid>,
    Query(params): Query<SearchMessagesQuery>,
) -> Result<Json<Vec<MessageResponse>>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();
    match conversation_messages::search_messages(&pool, conversation_id, &params).await {
        Ok(messages) => Ok(Json(messages)),
        Err(e) => Err(db_err(e)),
    }
}

pub async fn start_call(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<Uuid>,
    Json(req): Json<StartCallRequest>,
) -> Result<Json<CallResponse>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();
    match conversation_calls::start_call(&pool, conversation_id, &req).await {
        Ok(c) => Ok(Json(c)),
        Err(e) => Err(db_err(e)),
    }
}

pub async fn join_call(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(conversation_id): Path<Uuid>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();
    let user_id = if user.user_id == Uuid::nil() { None } else { Some(user.user_id) };
    match conversation_calls::join_call(&pool, conversation_id, user_id).await {
        Ok(r) => Ok(Json(r)),
        Err(e) => Err(db_err(e)),
    }
}

pub async fn leave_call(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(conversation_id): Path<Uuid>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();
    let user_id = if user.user_id == Uuid::nil() { None } else { Some(user.user_id) };
    match conversation_calls::leave_call(&pool, conversation_id, user_id).await {
        Ok(r) => Ok(Json(r)),
        Err(e) => Err(db_err(e)),
    }
}

pub async fn mute_call(
    State(_state): State<Arc<AppState>>,
    Path(_conversation_id): Path<Uuid>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Mute state is client-side WebRTC state, not persisted server-side.
    Ok(Json(SuccessResponse {
        success: true,
        message: Some("Audio muted".to_string()),
    }))
}

pub async fn unmute_call(
    State(_state): State<Arc<AppState>>,
    Path(_conversation_id): Path<Uuid>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Mute state is client-side WebRTC state, not persisted server-side.
    Ok(Json(SuccessResponse {
        success: true,
        message: Some("Audio unmuted".to_string()),
    }))
}

pub async fn start_screen_share(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(conversation_id): Path<Uuid>,
    Json(req): Json<ScreenShareRequest>,
) -> Result<Json<ScreenShareResponse>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();
    match conversation_calls::start_screen_share(&pool, conversation_id, &req, user.user_id).await {
        Ok(s) => Ok(Json(s)),
        Err(e) => Err(db_err(e)),
    }
}

pub async fn stop_screen_share(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<Uuid>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();
    match conversation_calls::stop_screen_share(&pool, conversation_id).await {
        Ok(r) => Ok(Json(r)),
        Err(e) => Err(db_err(e)),
    }
}

pub async fn start_recording(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<Uuid>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();
    match conversation_calls::toggle_recording(&pool, conversation_id, true).await {
        Ok(r) => Ok(Json(r)),
        Err(e) => Err(db_err(e)),
    }
}

pub async fn stop_recording(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<Uuid>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();
    match conversation_calls::toggle_recording(&pool, conversation_id, false).await {
        Ok(r) => Ok(Json(r)),
        Err(e) => Err(db_err(e)),
    }
}

pub async fn create_whiteboard(
    State(_state): State<Arc<AppState>>,
    Path(_conversation_id): Path<Uuid>,
) -> Result<Json<WhiteboardResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Whiteboards are served by the dedicated WebSocket whiteboard service
    // (/whiteboard/create/:conversation_id); this REST alias has no backing
    // store of its own and returns an honest 501.
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": "Whiteboard REST endpoint not implemented",
            "details": "Use the WebSocket whiteboard service at /whiteboard/create/:conversation_id"
        })),
    ))
}

pub async fn collaborate_whiteboard(
    State(_state): State<Arc<AppState>>,
    Path(_conversation_id): Path<Uuid>,
    Json(_data): Json<serde_json::Value>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Collaboration happens over the WebSocket whiteboard service.
    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": "Whiteboard collaboration REST endpoint not implemented",
            "details": "Use the WebSocket whiteboard service at /whiteboard/create/:conversation_id"
        })),
    ))
}
