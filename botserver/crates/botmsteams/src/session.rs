use crate::adapter::TeamsAdapter;
use crate::channel::ChannelAdapter;
use crate::schema::user_sessions::dsl::*;
use crate::state::{AttendantNotification, ChannelState};
use botlib::models::BotResponse;
use chrono::Utc;
use diesel::prelude::*;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = crate::schema::user_sessions)]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: String,
    pub bot_id: Uuid,
    pub title: String,
    pub answer_mode: i32,
    pub context_data: serde_json::Value,
    pub current_tool: Option<String>,
    pub message_count: i32,
    pub total_tokens: i32,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub last_activity: chrono::DateTime<Utc>,
    pub tenant_id: Option<Uuid>,
    pub active_email_account_id: Option<Uuid>,
    pub branch_id: Uuid,
    pub session_id: Option<String>,
    pub data: Option<serde_json::Value>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
}

pub fn find_or_create_session(
    state: &Arc<ChannelState>,
    conversation_id: &str,
    user_name: &str,
) -> Result<UserSession, Box<dyn std::error::Error + Send + Sync>> {
    let mut conn = state.conn.get()?;

    let teams_user_uuid =
        Uuid::new_v5(&Uuid::NAMESPACE_OID, format!("teams:{}", conversation_id).as_bytes());

    let teams_user_id = teams_user_uuid.to_string();

    let existing: Option<UserSession> = user_sessions
        .filter(user_id.eq(&teams_user_id))
        .order(crate::schema::user_sessions::updated_at.desc())
        .first(&mut conn)
        .optional()?;

    if let Some(session) = existing {
        diesel::update(user_sessions.filter(crate::schema::user_sessions::id.eq(session.id)))
            .set(crate::schema::user_sessions::updated_at.eq(Utc::now()))
            .execute(&mut conn)?;
        return Ok(session);
    }

    let bot_uuid = (state.get_default_bot)(&mut conn).0;
    let session_uuid = Uuid::new_v4();

    let context = serde_json::json!({
        "channel": "teams",
        "conversation_id": conversation_id,
        "name": user_name,
    });

    let now = Utc::now();

    diesel::insert_into(user_sessions)
        .values((
            crate::schema::user_sessions::id.eq(session_uuid),
            crate::schema::user_sessions::branch_id.eq(Uuid::nil()),
            crate::schema::user_sessions::bot_id.eq(bot_uuid),
            crate::schema::user_sessions::session_id.eq(session_uuid.to_string()),
            crate::schema::user_sessions::user_id.eq(&teams_user_id),
            crate::schema::user_sessions::data.eq(None::<serde_json::Value>),
            crate::schema::user_sessions::expires_at.eq(None::<chrono::DateTime<Utc>>),
            crate::schema::user_sessions::created_at.eq(now),
            crate::schema::user_sessions::updated_at.eq(now),
            crate::schema::user_sessions::title.eq(format!("Teams: {}", user_name)),
            crate::schema::user_sessions::context_data.eq(&context),
            crate::schema::user_sessions::current_tool.eq(None::<String>),
        ))        
        .execute(&mut conn)?;

    info!(
        "Created new Teams session {} for conversation_id {}",
        session_uuid, conversation_id
    );

    let new_session = user_sessions
        .filter(crate::schema::user_sessions::id.eq(session_uuid))
        .first(&mut conn)?;

    Ok(new_session)
}

pub async fn route_to_bot(
    state: Arc<ChannelState>,
    session: &UserSession,
    content: &str,
    conversation_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Routing Teams message to bot for session {}", session.id);

    let user_message = botlib::models::UserMessage::text(
        session.bot_id.to_string(),
        conversation_id.to_string(),
        session.id.to_string(),
        "teams".to_string(),
        content.to_string(),
    );

    let (tx, mut rx) = tokio::sync::mpsc::channel::<BotResponse>(10);
    let handle = (state.stream_response)(user_message, tx);

    let adapter = TeamsAdapter::new(
        state.conn.clone(),
        session.bot_id,
        state.get_config.clone(),
    );
    let conv_id_clone = conversation_id.to_string();

    tokio::spawn(async move {
        let mut accumulated_content = String::new();
        let mut chunk_count = 0u32;

        while let Some(response) = rx.recv().await {
            if !response.content.is_empty() {
                accumulated_content.push_str(&response.content);
                chunk_count += 1;
            }

            if (response.is_complete || chunk_count >= 5)
                && !accumulated_content.is_empty() {
                    let teams_response = BotResponse::new(
                        response.bot_id,
                        response.session_id,
                        conv_id_clone.clone(),
                        accumulated_content.clone(),
                        "teams",
                    );

                    if let Err(e) = adapter.send_bot_response(teams_response).await {
                        log::error!("Failed to send Teams response: {}", e);
                    }

                    accumulated_content.clear();
                    chunk_count = 0;
                }
        }
    });

    if let Err(e) = handle.await {
        log::error!("Bot processing error: {:?}", e);

        let adapter = TeamsAdapter::new(
            state.conn.clone(),
            session.bot_id,
            state.get_config.clone(),
        );
        let error_response = BotResponse::new(
            session.bot_id.to_string(),
            session.id.to_string(),
            conversation_id.to_string(),
            "Sorry, I encountered an error processing your message. Please try again.",
            "teams",
        );

        if let Err(e) = adapter.send_bot_response(error_response).await {
            log::error!("Failed to send error response: {}", e);
        }
    }

    Ok(())
}

pub fn route_to_attendant(
    state: Arc<ChannelState>,
    session: &UserSession,
    content: &str,
    conversation_id: &str,
    user_name: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "Routing Teams message to attendant for session {}",
        session.id
    );

    let assigned_to = session
        .context_data
        .get("assigned_to")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let notification = AttendantNotification {
        notification_type: "message".to_string(),
        session_id: session.id.to_string(),
        user_id: conversation_id.to_string(),
        user_name: Some(user_name.to_string()),
        user_phone: None,
        channel: "teams".to_string(),
        content: content.to_string(),
        timestamp: Utc::now().to_rfc3339(),
        assigned_to,
        priority: 1,
    };

    if let Some(broadcast_tx) = state.attendant_broadcast.as_ref() {
        if let Err(e) = broadcast_tx.send(notification.clone()) {
            debug!("No attendants listening: {}", e);
        } else {
            info!("Notification sent to attendants");
        }
    }

    Ok(())
}
