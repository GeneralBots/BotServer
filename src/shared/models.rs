use chrono::Utc;
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

#[derive(Debug, Clone, Queryable, Serialize, Deserialize)]
#[diesel(table_name = users)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
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
    pub param: String,
    pub is_active: bool,
    pub last_triggered: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = user_sessions)]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub bot_id: Uuid,
    pub title: String,
    pub context_data: serde_json::Value,
    pub answer_mode: i32,
    pub current_tool: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
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
    pub message_type: i32,
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
    pub message_type: i32,
    pub stream_token: Option<String>,
    pub is_complete: bool,
}

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Insertable)]
#[diesel(table_name = bot_memories)]
pub struct BotMemory {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub key: String,
    pub value: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Insertable)]
#[diesel(table_name = kb_documents)]
pub struct KBDocument {
    pub id: String,
    pub bot_id: String,
    pub user_id: String,
    pub collection_name: String,
    pub file_path: String,
    pub file_size: i32,
    pub file_hash: String,
    pub first_published_at: String,
    pub last_modified_at: String,
    pub indexed_at: Option<String>,
    pub metadata: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Insertable)]
#[diesel(table_name = basic_tools)]
pub struct BasicTool {
    pub id: String,
    pub bot_id: String,
    pub tool_name: String,
    pub file_path: String,
    pub ast_path: String,
    pub file_hash: String,
    pub mcp_json: Option<String>,
    pub tool_json: Option<String>,
    pub compiled_at: String,
    pub is_active: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Insertable)]
#[diesel(table_name = kb_collections)]
pub struct KBCollection {
    pub id: String,
    pub bot_id: String,
    pub user_id: String,
    pub name: String,
    pub folder_path: String,
    pub qdrant_collection: String,
    pub document_count: i32,
    pub is_active: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Insertable)]
#[diesel(table_name = user_kb_associations)]
pub struct UserKBAssociation {
    pub id: String,
    pub user_id: String,
    pub bot_id: String,
    pub kb_name: String,
    pub is_website: i32,
    pub website_url: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Insertable)]
#[diesel(table_name = session_tool_associations)]
pub struct SessionToolAssociation {
    pub id: String,
    pub session_id: String,
    pub tool_name: String,
    pub added_at: String,
}

pub mod schema {
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
            answer_mode -> Int4,
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
            role -> Int4,
            content_encrypted -> Text,
            message_type -> Int4,
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

    diesel::table! {
        bot_memories (id) {
            id -> Uuid,
            bot_id -> Uuid,
            key -> Text,
            value -> Text,
            created_at -> Timestamptz,
            updated_at -> Timestamptz,
        }
    }

    diesel::table! {
        kb_documents (id) {
            id -> Text,
            bot_id -> Text,
            user_id -> Text,
            collection_name -> Text,
            file_path -> Text,
            file_size -> Integer,
            file_hash -> Text,
            first_published_at -> Text,
            last_modified_at -> Text,
            indexed_at -> Nullable<Text>,
            metadata -> Text,
            created_at -> Text,
            updated_at -> Text,
        }
    }

    diesel::table! {
        basic_tools (id) {
            id -> Text,
            bot_id -> Text,
            tool_name -> Text,
            file_path -> Text,
            ast_path -> Text,
            file_hash -> Text,
            mcp_json -> Nullable<Text>,
            tool_json -> Nullable<Text>,
            compiled_at -> Text,
            is_active -> Integer,
            created_at -> Text,
            updated_at -> Text,
        }
    }

    diesel::table! {
        kb_collections (id) {
            id -> Text,
            bot_id -> Text,
            user_id -> Text,
            name -> Text,
            folder_path -> Text,
            qdrant_collection -> Text,
            document_count -> Integer,
            is_active -> Integer,
            created_at -> Text,
            updated_at -> Text,
        }
    }

    diesel::table! {
        user_kb_associations (id) {
            id -> Text,
            user_id -> Text,
            bot_id -> Text,
            kb_name -> Text,
            is_website -> Integer,
            website_url -> Nullable<Text>,
            created_at -> Text,
            updated_at -> Text,
        }
    }

    diesel::table! {
        session_tool_associations (id) {
            id -> Text,
            session_id -> Text,
            tool_name -> Text,
            added_at -> Text,
        }
    }
}

// Re-export all tables at the module level for backward compatibility
pub use schema::*;
