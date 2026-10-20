use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::{
    bot_configuration, bot_memories, bots, clicks, message_history, organizations,
    system_automations, tenants, user_login_tokens, user_preferences, user_sessions, users,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerKind {
    Scheduled = 0,
    TableUpdate = 1,
    TableInsert = 2,
    TableDelete = 3,
    Webhook = 4,
    EmailReceived = 5,
    FolderChange = 6,
    DealStageChange = 7,
    ContactChange = 8,
    EmailOpened = 9,
    /// A media file arrived on any channel (Telegram, WhatsApp, web) and was
    /// staged in the bot's Drive `inbox/`. `target` is the event name,
    /// `param` the tool to run — same convention as `Webhook`/`EmailReceived`.
    MediaUploaded = 10,
    /// A text message arrived on any channel. `target` is the event name.
    MessageReceived = 11,
}

impl TriggerKind {
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Scheduled),
            1 => Some(Self::TableUpdate),
            2 => Some(Self::TableInsert),
            3 => Some(Self::TableDelete),
            4 => Some(Self::Webhook),
            5 => Some(Self::EmailReceived),
            6 => Some(Self::FolderChange),
            7 => Some(Self::DealStageChange),
            8 => Some(Self::ContactChange),
            9 => Some(Self::EmailOpened),
            10 => Some(Self::MediaUploaded),
            11 => Some(Self::MessageReceived),
            _ => None,
        }
    }
}

#[derive(Debug, Queryable, Serialize, Deserialize, Identifiable)]
#[diesel(table_name = system_automations)]
pub struct Automation {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub kind: i32,
    pub target: Option<String>,
    pub schedule: Option<String>,
    pub param: String,
    pub is_active: bool,
    pub last_triggered: Option<DateTime<Utc>>,
    pub branch_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = user_sessions)]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub bot_id: Uuid,
    pub title: String,
    pub context_data: serde_json::Value,
    pub current_tool: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Insertable)]
#[diesel(table_name = bot_memories)]
pub struct BotMemory {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub key: String,
    pub value: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// NOT NULL since `6.5.23-branch-scope-cleanup`.
    pub branch_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = users)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub phone_number: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: Option<bool>,
    pub is_admin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = bots)]
pub struct Bot {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub llm_provider: String,
    pub llm_config: serde_json::Value,
    pub context_provider: String,
    pub context_config: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: Option<bool>,
    pub org_id: Option<Uuid>,
    pub database_name: Option<String>,
    pub is_public: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = tenants)]
#[diesel(primary_key(id))]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Queryable, Serialize, Deserialize, Identifiable)]
#[diesel(table_name = organizations)]
#[diesel(primary_key(org_id))]
pub struct Organization {
    pub org_id: Uuid,
    pub name: String,
    pub slug: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tenant_id: Option<Uuid>,
    pub domain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = message_history)]
pub struct MessageHistory {
    pub id: Uuid,
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub role: i32,
    pub content_encrypted: String,
    pub message_type: i32,
    pub message_index: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = bot_configuration)]
pub struct BotConfiguration {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub config_key: String,
    pub config_value: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub branch_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = user_login_tokens)]
pub struct UserLoginToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = user_preferences)]
pub struct UserPreference {
    pub id: Uuid,
    pub user_id: Uuid,
    pub preference_key: String,
    pub preference_value: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = clicks)]
pub struct Click {
    pub id: Uuid,
    pub campaign_id: String,
    pub email: String,
    pub updated_at: DateTime<Utc>,
}
