use crate::adapter::TelegramAdapter;
use crate::channel::ChannelAdapter;
use crate::schema::user_sessions;
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

/// Session for the per-bot webhook route: the bot comes from the URL path
/// instead of the workspace default.
pub fn find_or_create_session_for_bot(
    state: &Arc<ChannelState>,
    bot_id: Uuid,
    chat_id: &str,
    user_name: &str,
) -> Result<UserSession, Box<dyn std::error::Error + Send + Sync>> {
    find_or_create_session_scoped(state, Some(bot_id), chat_id, user_name)
}

/// Scoped session lookup/creation for Telegram deliveries. With `bot_id`
/// (per-bot webhook route) the session is bound to that bot; without it the
/// workspace default bot is resolved. An existing session is reused only when
/// it is bound to the same bot — a Telegram chat id belongs to one bot, so a
/// binding mismatch means the chat must get a session for the bot the
/// delivery actually targets.
fn find_or_create_session_scoped(
    state: &Arc<ChannelState>,
    bot_id: Option<Uuid>,
    chat_id: &str,
    user_name: &str,
) -> Result<UserSession, Box<dyn std::error::Error + Send + Sync>> {
    let mut conn = state.conn.get()?;

    let telegram_user_uuid =
        Uuid::new_v5(&Uuid::NAMESPACE_OID, format!("telegram:{}", chat_id).as_bytes());

    let telegram_user_id = telegram_user_uuid.to_string();

    let mut existing_query = user_sessions::table
        .filter(user_sessions::user_id.eq(&telegram_user_id))
        .into_boxed();
    if let Some(bot_id) = bot_id {
        existing_query = existing_query.filter(user_sessions::bot_id.eq(bot_id));
    }
    let existing: Option<UserSession> = existing_query
        .order(crate::schema::user_sessions::updated_at.desc())
        .first(&mut conn)
        .optional()?;

    if let Some(session) = existing {
        diesel::update(
            user_sessions::table.filter(crate::schema::user_sessions::id.eq(session.id)),
        )
            .set(crate::schema::user_sessions::updated_at.eq(Utc::now()))
            .execute(&mut conn)?;
        return Ok(session);
    }

    let (bot_uuid, branch_uuid) = match bot_id {
        Some(bot_id) => {
            let branch = resolve_branch_for_bot(&mut conn, bot_id);
            (bot_id, branch)
        }
        None => resolve_bot_scope(state, &mut conn),
    };
    let session_uuid = Uuid::new_v4();

    let context = serde_json::json!({
        "channel": "telegram",
        "chat_id": chat_id,
        "name": user_name,
    });

    let now = Utc::now();

    diesel::insert_into(user_sessions::table)
        .values((
            crate::schema::user_sessions::id.eq(session_uuid),
            crate::schema::user_sessions::branch_id.eq(branch_uuid),
            crate::schema::user_sessions::bot_id.eq(bot_uuid),
            crate::schema::user_sessions::session_id.eq(session_uuid.to_string()),
            crate::schema::user_sessions::user_id.eq(&telegram_user_id),
            crate::schema::user_sessions::data.eq(None::<serde_json::Value>),
            crate::schema::user_sessions::expires_at.eq(None::<chrono::DateTime<Utc>>),
            crate::schema::user_sessions::created_at.eq(now),
            crate::schema::user_sessions::updated_at.eq(now),
            crate::schema::user_sessions::title.eq(format!("Telegram: {}", user_name)),
            crate::schema::user_sessions::context_data.eq(&context),
            crate::schema::user_sessions::current_tool.eq(None::<String>),
        ))
        .execute(&mut conn)?;

    info!(
        "Created new Telegram session {} for chat_id {}",
        session_uuid, chat_id
    );

    let new_session = user_sessions::table
        .filter(crate::schema::user_sessions::id.eq(session_uuid))
        .first(&mut conn)?;

    Ok(new_session)
}

#[derive(Debug, diesel::QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct BotScopeRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    branch_id: Uuid,
}

/// Resolves the bot and branch backing a channel session.
///
/// The channel wiring hands the adapter the workspace's default branch as the
/// bot handle, so the row is looked up by bot id or by branch id. Without this
/// lookup the session stored the handle as its `bot_id` and `Uuid::nil()` as its
/// `branch_id`, leaving channel work unscoped.
pub(crate) fn resolve_bot_scope(state: &Arc<ChannelState>, conn: &mut PgConnection) -> (Uuid, Uuid) {
    let handle = (state.get_default_bot)(conn);

    if handle.0.is_nil() {
        return (handle.0, Uuid::nil());
    }

    let row = diesel::sql_query(
        "SELECT id, branch_id FROM bots WHERE id = $1 OR branch_id = $1 LIMIT 1",
    )
    .bind::<diesel::sql_types::Uuid, _>(handle.0)
    .get_result::<BotScopeRow>(&mut *conn)
    .optional();

    match row {
        Ok(Some(row)) => (row.id, row.branch_id),
        Ok(None) => (handle.0, Uuid::nil()),
        Err(e) => {
            log::warn!(
                "Could not resolve the bot behind channel handle {}: {e}",
                handle.0
            );
            (handle.0, Uuid::nil())
        }
    }
}

/// Resolves a bot by name or slug for the per-bot webhook route. Returns
/// `None` when no bot carries the name: the delivery is then rejected instead
/// of falling back to the workspace default bot, which would attribute the
/// message to the wrong conversation.
pub(crate) fn resolve_bot_scope_by_name(
    conn: &mut PgConnection,
    bot_name: &str,
) -> Option<(Uuid, Uuid)> {
    diesel::sql_query("SELECT id, branch_id FROM bots WHERE name = $1 OR slug = $1 LIMIT 1")
        .bind::<diesel::sql_types::Text, _>(bot_name.to_string())
        .get_result::<BotScopeRow>(conn)
        .optional()
        .ok()
        .flatten()
        .map(|row| (row.id, row.branch_id))
}

/// Branch of the bot row, for sessions created through the per-bot webhook
/// route where the bot is already known and only the branch scope is missing.
fn resolve_branch_for_bot(conn: &mut PgConnection, bot_id: Uuid) -> Uuid {
    diesel::sql_query("SELECT id, branch_id FROM bots WHERE id = $1 LIMIT 1")
        .bind::<diesel::sql_types::Uuid, _>(bot_id)
        .get_result::<BotScopeRow>(conn)
        .optional()
        .ok()
        .flatten()
        .map(|row| row.branch_id)
        .unwrap_or(Uuid::nil())
}

pub async fn route_to_bot(
    state: Arc<ChannelState>,
    session: &UserSession,
    content: &str,
    chat_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Routing Telegram message to bot for session {}", session.id);

    let user_message = botlib::models::UserMessage::text(
        session.bot_id.to_string(),
        chat_id.to_string(),
        session.id.to_string(),
        "telegram".to_string(),
        content.to_string(),
    );

    let (tx, mut rx) = tokio::sync::mpsc::channel::<BotResponse>(10);
    let handle = (state.stream_response)(user_message, tx);

    let adapter = TelegramAdapter::new(
        state.conn.clone(),
        session.bot_id,
        state.get_config.clone(),
    );
    let chat_id_clone = chat_id.to_string();

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
                    let tg_response = BotResponse::new(
                        response.bot_id,
                        response.session_id,
                        chat_id_clone.clone(),
                        accumulated_content.clone(),
                        "telegram",
                    );

                    if let Err(e) = adapter.send_bot_response(tg_response).await {
                        log::error!("Failed to send Telegram response: {}", e);
                    }

                    accumulated_content.clear();
                    chunk_count = 0;
                }
        }
    });

    if let Err(e) = handle.await {
        log::error!("Bot processing error: {:?}", e);

        let adapter = TelegramAdapter::new(
            state.conn.clone(),
            session.bot_id,
            state.get_config.clone(),
        );
        let error_response = BotResponse::new(
            session.bot_id.to_string(),
            session.id.to_string(),
            chat_id.to_string(),
            "Sorry, I encountered an error processing your message. Please try again.",
            "telegram",
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
    chat_id: &str,
    user_name: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "Routing Telegram message to attendant for session {}",
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
        user_id: chat_id.to_string(),
        user_name: Some(user_name.to_string()),
        user_phone: Some(chat_id.to_string()),
        channel: "telegram".to_string(),
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
