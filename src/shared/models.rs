use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable)]
#[diesel(table_name = organizations)]
pub struct Organization {
    pub org_id: Uuid,
    pub name: String,
    pub slug: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable)]
#[diesel(table_name = bots)]
pub struct Bot {
    pub bot_id: Uuid,
    pub name: String,
    pub status: i32,
    pub config: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
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

#[derive(Debug, Queryable, Serialize, Deserialize, Identifiable)]
#[diesel(table_name = system_automations)]
pub struct Automation {
    pub id: Uuid,
    pub kind: i32,
    pub target: Option<String>,
    pub schedule: Option<String>,
    pub script_name: String,
    pub param: String,
    pub is_active: bool,
    pub last_triggered: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = user_sessions)]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub bot_id: Uuid,
    pub title: String,
    pub context_data: serde_json::Value,
    pub answer_mode: String,
    pub current_tool: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
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
    pub timestamp: chrono::DateTime<chrono::Utc>,
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

diesel::table! {
    organizations (org_id) {
        org_id -> Uuid,
        name -> Text,
        slug -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    bots (bot_id) {
        bot_id -> Uuid,
        name -> Text,
        status -> Int4,
        config -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    system_automations (id) {
        id -> Uuid,
        kind -> Int4,
        target -> Nullable<Text>,
        schedule -> Nullable<Text>,
        script_name -> Text,
        param -> Text,
        is_active -> Bool,
        last_triggered -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    user_sessions (id) {
        id -> Uuid,
        user_id -> Uuid,
        bot_id -> Uuid,
        title -> Text,
        context_data -> Jsonb,
        answer_mode -> Text,
        current_tool -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    message_history (id) {
        id -> Uuid,
        session_id -> Uuid,
        user_id -> Uuid,
        role -> Text,
        content_encrypted -> Text,
        message_type -> Text,
        message_index -> Int8,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        username -> Text,
        email -> Text,
        password_hash -> Text,
        is_active -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    clicks (id) {
        id -> Uuid,
        campaign_id -> Text,
        email -> Text,
        updated_at -> Timestamptz,
    }
}
