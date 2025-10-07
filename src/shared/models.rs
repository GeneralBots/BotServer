use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Organization {
    pub org_id: Uuid,
    pub name: String,
    pub slug: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Bot {
    pub bot_id: Uuid,
    pub name: String,
    pub status: i32,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum BotStatus {
    Active,
    Inactive,
    Maintenance,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TriggerKind {
    Scheduled = 0,
    TableUpdate = 1,
    TableInsert = 2,
    TableDelete = 3,
}

impl TriggerKind {
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Scheduled),
            1 => Some(Self::TableUpdate),
            2 => Some(Self::TableInsert),
            3 => Some(Self::TableDelete),
            _ => None,
        }
    }
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Automation {
    pub id: Uuid,
    pub kind: i32,
    pub target: Option<String>,
    pub schedule: Option<String>,
    pub param: String,
    pub is_active: bool,
    pub last_triggered: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub bot_id: Uuid,
    pub title: String,
    pub context_data: serde_json::Value,
    pub answer_mode: String,
    pub current_tool: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub text: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub embedding: Vec<f32>,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub text: String,
    pub similarity: f32,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub bot_id: String,
    pub user_id: String,
    pub session_id: String,
    pub channel: String,
    pub content: String,
    pub message_type: String,
    pub media_url: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotResponse {
    pub bot_id: String,
    pub user_id: String,
    pub session_id: String,
    pub channel: String,
    pub content: String,
    pub message_type: String,
    pub stream_token: Option<String>,
    pub is_complete: bool,
}

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}
