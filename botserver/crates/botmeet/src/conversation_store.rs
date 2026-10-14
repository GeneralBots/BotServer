//! DB-backed store for standalone meet group conversations.
//!
//! Backs the `/conversations/*` endpoints. Each operation is a small bound
//! SQL statement scoped by `conversation_id`; callers pass a `DbPool` and run
//! the async fns inside `tokio::task::spawn_blocking`.

use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::{Bool, Nullable, Text, Timestamptz, Uuid as DieselUuid};
use uuid::Uuid;

use crate::conversations::{
    ConversationResponse, CreateConversationRequest, JoinConversationRequest,
    LeaveConversationRequest, ParticipantResponse, SuccessResponse,
};

pub use botlib::db_pool::DbPool;
pub type PgConnection = diesel::PgConnection;

/// Resolves a nil bot id to the default bot so requests without an explicit
/// bot still persist to a real row (mirrors the pattern in botautotask).
pub fn resolve_effective_bot_id(pool: &DbPool, bot_id: Uuid) -> Uuid {
    if bot_id != Uuid::nil() {
        return bot_id;
    }
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return bot_id,
    };
    #[derive(diesel::QueryableByName)]
    struct BotIdRow {
        #[diesel(sql_type = DieselUuid)]
        id: Uuid,
    }
    sql_query("SELECT id FROM bots WHERE name = 'default' AND is_active = true LIMIT 1")
        .get_result::<BotIdRow>(&mut conn)
        .optional()
        .ok()
        .flatten()
        .map(|r| r.id)
        .unwrap_or(bot_id)
}

pub async fn create_conversation(
    pool: &DbPool,
    bot_id: Uuid,
    req: &CreateConversationRequest,
) -> Result<ConversationResponse> {
    let bot_id = resolve_effective_bot_id(pool, bot_id);
    let now = Utc::now();
    let conversation_id = Uuid::new_v4();
    let creator_id = Uuid::new_v4();

    let conversation_type = req.conversation_type.clone().unwrap_or_else(|| "group".to_string());
    let is_private = req.is_private.unwrap_or(false);

    let mut conn = pool.get()?;
    sql_query(
        "INSERT INTO meet_conversations (id, bot_id, name, description, conversation_type, is_private, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind::<DieselUuid, _>(conversation_id)
    .bind::<DieselUuid, _>(bot_id)
    .bind::<Text, _>(&req.name)
    .bind::<Nullable<Text>, _>(req.description.as_deref())
    .bind::<Text, _>(&conversation_type)
    .bind::<Bool, _>(is_private)
    .bind::<Nullable<DieselUuid>, _>(Some(creator_id))
    .execute(&mut conn)?;

    for participant in &req.participants {
        sql_query(
            "INSERT INTO meet_conversation_members (conversation_id, user_id, role)
             VALUES ($1, $2, 'member') ON CONFLICT (conversation_id, user_id) DO NOTHING",
        )
        .bind::<DieselUuid, _>(conversation_id)
        .bind::<DieselUuid, _>(*participant)
        .execute(&mut conn)?;
    }

    Ok(ConversationResponse {
        id: conversation_id,
        name: req.name.clone(),
        description: req.description.clone(),
        conversation_type,
        is_private,
        participant_count: req.participants.len() as u32,
        unread_count: 0,
        created_by: creator_id,
        created_at: now,
        updated_at: now,
        last_message: None,
    })
}

pub async fn join_conversation(
    pool: &DbPool,
    conversation_id: Uuid,
    req: &JoinConversationRequest,
) -> Result<SuccessResponse> {
    let mut conn = pool.get()?;
    sql_query(
        "INSERT INTO meet_conversation_members (conversation_id, user_id, display_name, role, status)
         VALUES ($1, $2, $3, 'member', 'active')
         ON CONFLICT (conversation_id, user_id)
         DO UPDATE SET status = 'active', left_at = NULL, joined_at = NOW()",
    )
    .bind::<DieselUuid, _>(conversation_id)
    .bind::<DieselUuid, _>(req.user_id)
    .bind::<Nullable<Text>, _>(req.display_name.as_deref())
    .execute(&mut conn)?;

    Ok(SuccessResponse {
        success: true,
        message: Some(format!("User {} joined conversation {}", req.user_id, conversation_id)),
    })
}

pub async fn leave_conversation(
    pool: &DbPool,
    conversation_id: Uuid,
    req: &LeaveConversationRequest,
) -> Result<SuccessResponse> {
    let mut conn = pool.get()?;
    sql_query(
        "UPDATE meet_conversation_members SET status = 'left', left_at = NOW()
         WHERE conversation_id = $1 AND user_id = $2",
    )
    .bind::<DieselUuid, _>(conversation_id)
    .bind::<DieselUuid, _>(req.user_id)
    .execute(&mut conn)?;

    Ok(SuccessResponse {
        success: true,
        message: Some(format!("User {} left conversation {}", req.user_id, conversation_id)),
    })
}

#[derive(diesel::QueryableByName)]
struct MemberRow {
    #[diesel(sql_type = DieselUuid)]
    user_id: Uuid,
    #[diesel(sql_type = Nullable<Text>)]
    display_name: Option<String>,
    #[diesel(sql_type = Text)]
    role: String,
    #[diesel(sql_type = Text)]
    status: String,
    #[diesel(sql_type = Timestamptz)]
    joined_at: DateTime<Utc>,
}

pub async fn get_conversation_members(pool: &DbPool, conversation_id: Uuid) -> Result<Vec<ParticipantResponse>> {
    let mut conn = pool.get()?;
    let rows: Vec<MemberRow> = sql_query(
        "SELECT user_id, display_name, role, status, joined_at FROM meet_conversation_members
         WHERE conversation_id = $1 AND status = 'active' ORDER BY joined_at",
    )
    .bind::<DieselUuid, _>(conversation_id)
    .get_results(&mut conn)?;

    Ok(rows
        .into_iter()
        .map(|r| ParticipantResponse {
            user_id: r.user_id,
            username: r.display_name.clone().unwrap_or_else(|| r.user_id.to_string()),
            display_name: r.display_name,
            role: r.role,
            status: r.status,
            joined_at: r.joined_at,
            is_typing: false,
        })
        .collect())
}
