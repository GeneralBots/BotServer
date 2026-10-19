use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::shared::schema::{
    branches, kb_collections, kb_group_associations,
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
    pub bot_id: Option<Uuid>,
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

impl From<UserSession> for botbasic_types::UserSession {
    fn from(s: UserSession) -> Self {
        Self {
            id: s.id,
            user_id: s.user_id,
            branch_id: Uuid::nil(),
            bot_id: s.bot_id,
            title: s.title,
            context_data: s.context_data,
            current_tool: s.current_tool,
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}

impl From<botbasic_types::UserSession> for UserSession {
    fn from(s: botbasic_types::UserSession) -> Self {
        Self {
            id: s.id,
            user_id: s.user_id,
            bot_id: s.bot_id,
            title: s.title,
            context_data: s.context_data,
            current_tool: s.current_tool,
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
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
    pub is_active: bool,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = bots)]
pub struct Bot {
    pub id: Uuid,
    pub org_id: Uuid,
    pub branch_id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub llm_provider: String,
    pub llm_config: serde_json::Value,
    pub context_provider: String,
    pub context_config: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: Option<bool>,
    pub database_name: Option<String>,
    pub is_public: bool,
    pub is_default_for_branch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = branches)]
pub struct Branch {
    pub id: Uuid,
    pub org_id: Uuid,
    pub tenant_id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Insertable, Serialize, Deserialize)]
#[diesel(table_name = branches)]
pub struct NewBranch {
    pub org_id: Uuid,
    pub tenant_id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = organizations)]
#[diesel(primary_key(org_id))]
pub struct Organization {
    pub org_id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub slug: String,
    pub created_at: DateTime<Utc>,
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
    pub is_encrypted: bool,
    pub config_type: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = kb_collections)]
pub struct KbCollection {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub folder_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = kb_group_associations)]
pub struct KbGroupAssociation {
    pub id: Uuid,
    pub collection_id: Uuid,
    pub group_id: Uuid,
    pub kb_id: Uuid,
    pub granted_at: DateTime<Utc>,
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

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = crate::shared::schema::organization_invitations)]
pub struct OrganizationInvitation {
    pub id: Uuid,
    pub org_id: Uuid,
    pub email: String,
    pub role: String,
    pub status: String,
    pub message: Option<String>,
    pub invited_by: Uuid,
    pub token: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub accepted_by: Option<Uuid>,
}
