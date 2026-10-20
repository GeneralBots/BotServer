use crate::schema::user_sessions;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Selectable)]
#[diesel(table_name = user_sessions)]
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub tenant_id: Option<Uuid>,
    pub active_email_account_id: Option<Uuid>,
    pub branch_id: Uuid,
    pub session_id: Option<String>,
    pub data: Option<serde_json::Value>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub data: String,
}
