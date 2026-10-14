//! Message operations for meet group conversations (send/edit/delete/react/pin/search).

use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::{Bool, Int8, Nullable, Text, Timestamptz, Uuid as DieselUuid};
use uuid::Uuid;

use crate::conversations::{
    EditMessageRequest, MessageResponse, SearchMessagesQuery,
    SendMessageRequest, SuccessResponse,
};

use crate::conversation_store::{DbPool, PgConnection};

#[derive(diesel::QueryableByName)]
struct MessageRow {
    #[diesel(sql_type = DieselUuid)]
    id: Uuid,
    #[diesel(sql_type = Nullable<DieselUuid>)]
    sender_id: Option<Uuid>,
    #[diesel(sql_type = Text)]
    sender_name: String,
    #[diesel(sql_type = Text)]
    message_type: String,
    #[diesel(sql_type = Text)]
    content: String,
    #[diesel(sql_type = Nullable<DieselUuid>)]
    reply_to: Option<Uuid>,
    #[diesel(sql_type = diesel::sql_types::Jsonb)]
    attachments: serde_json::Value,
    #[diesel(sql_type = diesel::sql_types::Jsonb)]
    reactions: serde_json::Value,
    #[diesel(sql_type = Bool)]
    is_pinned: bool,
    #[diesel(sql_type = Bool)]
    is_edited: bool,
    #[diesel(sql_type = Timestamptz)]
    created_at: DateTime<Utc>,
    #[diesel(sql_type = Timestamptz)]
    updated_at: DateTime<Utc>,
}

impl From<MessageRow> for MessageResponse {
    fn from(r: MessageRow) -> Self {
        MessageResponse {
            id: r.id,
            conversation_id: Uuid::nil(), // filled by caller
            sender_id: r.sender_id.unwrap_or(Uuid::nil()),
            sender_name: r.sender_name,
            content: r.content,
            message_type: r.message_type,
            reply_to: r.reply_to,
            attachments: serde_json::from_value(r.attachments).unwrap_or_default(),
            reactions: serde_json::from_value(r.reactions).unwrap_or_default(),
            is_pinned: r.is_pinned,
            is_edited: r.is_edited,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

const MESSAGE_SELECT: &str = "SELECT m.id, m.sender_id, m.sender_name, m.message_type, m.content, m.reply_to, m.attachments, \
    COALESCE(jsonb_agg(jsonb_build_object('user_id', r.user_id, 'reaction', r.reaction, 'timestamp', r.created_at)) \
    FILTER (WHERE r.id IS NOT NULL), '[]'::jsonb) AS reactions, \
    m.is_pinned, m.is_edited, m.created_at, m.updated_at \
    FROM meet_conversation_messages m \
    LEFT JOIN meet_conversation_reactions r ON r.message_id = m.id";

fn row_to_response(row: MessageRow, conversation_id: Uuid) -> MessageResponse {
    let mut msg: MessageResponse = row.into();
    msg.conversation_id = conversation_id;
    msg
}

pub async fn get_conversation_messages(pool: &DbPool, conversation_id: Uuid) -> Result<Vec<MessageResponse>> {
    let mut conn = pool.get()?;
    let rows: Vec<MessageRow> = sql_query(format!(
        "{MESSAGE_SELECT} WHERE m.conversation_id = $1 GROUP BY m.id ORDER BY m.created_at"
    ))
    .bind::<DieselUuid, _>(conversation_id)
    .get_results(&mut conn)?;

    Ok(rows.into_iter().map(|r| row_to_response(r, conversation_id)).collect())
}

pub async fn send_message(
    pool: &DbPool,
    conversation_id: Uuid,
    req: &SendMessageRequest,
    sender_id: Option<Uuid>,
    sender_name: Option<&str>,
) -> Result<MessageResponse> {
    let mut conn = pool.get()?;
    let message_id = Uuid::new_v4();
    let now = Utc::now();
    let sender_name = match (sender_id, sender_name) {
        (_, Some(name)) if !name.is_empty() => name.to_string(),
        (Some(sid), _) => sid.to_string(),
        _ => "User".to_string(),
    };
    let message_type = req.message_type.clone().unwrap_or_else(|| "text".to_string());
    let attachments = serde_json::to_value(req.attachments.clone().unwrap_or_default())?;

    sql_query(
        "INSERT INTO meet_conversation_messages
            (id, conversation_id, sender_id, sender_name, message_type, content, reply_to, attachments, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
    )
    .bind::<DieselUuid, _>(message_id)
    .bind::<DieselUuid, _>(conversation_id)
    .bind::<Nullable<DieselUuid>, _>(sender_id)
    .bind::<Text, _>(&sender_name)
    .bind::<Text, _>(&message_type)
    .bind::<Text, _>(&req.content)
    .bind::<Nullable<DieselUuid>, _>(req.reply_to)
    .bind::<diesel::sql_types::Jsonb, _>(&attachments)
    .bind::<Timestamptz, _>(now)
    .bind::<Timestamptz, _>(now)
    .execute(&mut conn)?;

    Ok(MessageResponse {
        id: message_id,
        conversation_id,
        sender_id: sender_id.unwrap_or(Uuid::nil()),
        sender_name,
        content: req.content.clone(),
        message_type,
        reply_to: req.reply_to,
        attachments: req.attachments.clone().unwrap_or_default(),
        reactions: vec![],
        is_pinned: false,
        is_edited: false,
        created_at: now,
        updated_at: now,
    })
}

pub async fn edit_message(
    pool: &DbPool,
    conversation_id: Uuid,
    message_id: Uuid,
    req: &EditMessageRequest,
) -> Result<MessageResponse> {
    let mut conn = pool.get()?;
    let now = Utc::now();
    sql_query(
        "UPDATE meet_conversation_messages SET content = $1, is_edited = TRUE, updated_at = $2
         WHERE id = $3 AND conversation_id = $4",
    )
    .bind::<Text, _>(&req.content)
    .bind::<Timestamptz, _>(now)
    .bind::<DieselUuid, _>(message_id)
    .bind::<DieselUuid, _>(conversation_id)
    .execute(&mut conn)?;

    let mut msg = load_message(&mut conn, message_id, conversation_id)?;
    msg.is_edited = true;
    msg.updated_at = now;
    Ok(msg)
}

pub async fn delete_message(pool: &DbPool, conversation_id: Uuid, message_id: Uuid) -> Result<SuccessResponse> {
    let mut conn = pool.get()?;
    sql_query("DELETE FROM meet_conversation_messages WHERE id = $1 AND conversation_id = $2")
        .bind::<DieselUuid, _>(message_id)
        .bind::<DieselUuid, _>(conversation_id)
        .execute(&mut conn)?;
    Ok(SuccessResponse {
        success: true,
        message: Some(format!("Message {} deleted", message_id)),
    })
}

pub async fn react_to_message(
    pool: &DbPool,
    conversation_id: Uuid,
    message_id: Uuid,
    user_id: Uuid,
    reaction: &str,
) -> Result<SuccessResponse> {
    let mut conn = pool.get()?;
    // Only allow reactions on messages that belong to the conversation
    let exists: i64 = sql_query(
        "SELECT COUNT(*) AS c FROM meet_conversation_messages WHERE id = $1 AND conversation_id = $2",
    )
    .bind::<DieselUuid, _>(message_id)
    .bind::<DieselUuid, _>(conversation_id)
    .get_result::<CountRow>(&mut conn)?
    .c;
    if exists == 0 {
        anyhow::bail!("Message not found in conversation");
    }
    sql_query(
        "INSERT INTO meet_conversation_reactions (message_id, user_id, reaction)
         VALUES ($1, $2, $3) ON CONFLICT (message_id, user_id, reaction) DO NOTHING",
    )
    .bind::<DieselUuid, _>(message_id)
    .bind::<DieselUuid, _>(user_id)
    .bind::<Text, _>(reaction)
    .execute(&mut conn)?;
    Ok(SuccessResponse {
        success: true,
        message: Some(format!("Reaction '{}' added to message {}", reaction, message_id)),
    })
}

pub async fn pin_message(pool: &DbPool, conversation_id: Uuid, message_id: Uuid) -> Result<SuccessResponse> {
    let mut conn = pool.get()?;
    sql_query("UPDATE meet_conversation_messages SET is_pinned = TRUE WHERE id = $1 AND conversation_id = $2")
        .bind::<DieselUuid, _>(message_id)
        .bind::<DieselUuid, _>(conversation_id)
        .execute(&mut conn)?;
    Ok(SuccessResponse {
        success: true,
        message: Some(format!("Message {} pinned", message_id)),
    })
}

pub async fn search_messages(pool: &DbPool, conversation_id: Uuid, params: &SearchMessagesQuery) -> Result<Vec<MessageResponse>> {
    let mut conn = pool.get()?;
    let pattern = format!("%{}%", params.query);
    let rows: Vec<MessageRow> = sql_query(format!(
        "{MESSAGE_SELECT} WHERE m.conversation_id = $1 AND m.content ILIKE $2 \
         GROUP BY m.id ORDER BY m.created_at LIMIT 100"
    ))
    .bind::<DieselUuid, _>(conversation_id)
    .bind::<Text, _>(&pattern)
    .get_results(&mut conn)?;

    Ok(rows.into_iter().map(|r| row_to_response(r, conversation_id)).collect())
}

#[derive(diesel::QueryableByName)]
struct CountRow {
    #[diesel(sql_type = Int8)]
    c: i64,
}

fn load_message(conn: &mut PgConnection, message_id: Uuid, conversation_id: Uuid) -> Result<MessageResponse> {
    let row: MessageRow = sql_query(format!(
        "{MESSAGE_SELECT} WHERE m.id = $1 AND m.conversation_id = $2 GROUP BY m.id"
    ))
    .bind::<DieselUuid, _>(message_id)
    .bind::<DieselUuid, _>(conversation_id)
    .get_result(conn)?;
    Ok(row_to_response(row, conversation_id))
}
