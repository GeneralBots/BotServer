use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = crate::schema::user_sessions)]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub branch_id: Uuid,
    pub bot_id: Uuid,
    pub title: String,
    pub context_data: serde_json::Value,
    pub current_tool: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = crate::schema::bot_memories)]
pub struct BotMemory {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub key: String,
    pub value: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotSharedMemory {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub key: String,
    pub value: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerKind(pub String);

impl TriggerKind {
    pub fn webhook() -> Self { Self("webhook".to_string()) }
    pub fn email_received() -> Self { Self("email_received".to_string()) }
    pub fn schedule() -> Self { Self("schedule".to_string()) }
    pub fn database_change() -> Self { Self("database_change".to_string()) }
    pub fn manual() -> Self { Self("manual".to_string()) }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub workflow_name: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEvent {
    pub id: Uuid,
    pub execution_id: Uuid,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub attachment_type: String,
    pub url: String,
    pub mime_type: Option<String>,
    pub filename: Option<String>,
    pub size: Option<u64>,
}

pub use botlib::db_pool::DbPool;

impl From<botlib::models::UserSession> for UserSession {
    fn from(u: botlib::models::UserSession) -> Self {
        Self { id: u.id, user_id: u.user_id, branch_id: u.branch_id, bot_id: u.bot_id, title: u.title, context_data: u.context_data, current_tool: u.current_tool, created_at: u.created_at, updated_at: u.updated_at }
    }
}
