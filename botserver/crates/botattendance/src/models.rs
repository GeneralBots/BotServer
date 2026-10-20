use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Queryable, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BotResponse {
    pub bot_id: String,
    pub session_id: String,
    pub user_id: String,
    pub channel: String,
    pub content: String,
    pub message_type: i32,
    pub stream_token: Option<String>,
    pub is_complete: bool,
    pub suggestions: Vec<String>,
    pub switchers: Vec<serde_json::Value>,
    pub context_name: Option<String>,
    pub context_length: i32,
    pub context_max_length: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttendantNotification {
    pub notification_type: String,
    pub session_id: String,
    pub user_id: String,
    pub user_name: Option<String>,
    pub user_phone: Option<String>,
    pub channel: String,
    pub content: String,
    pub timestamp: String,
    pub assigned_to: Option<String>,
    pub priority: i32,
}

#[derive(Debug, Deserialize)]
pub struct AttendantRespondRequest {
    pub session_id: String,
    pub message: String,
    pub attendant_id: String,
}

#[derive(Debug, Serialize)]
pub struct AttendantRespondResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
