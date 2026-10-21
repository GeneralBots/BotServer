use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::*;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = attendant_queues)]
pub struct AttendantQueue {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub priority: i32,
    pub max_wait_minutes: i32,
    pub auto_assign: bool,
    pub working_hours: serde_json::Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub branch_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = attendant_sessions)]
pub struct AttendantSession {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub session_number: String,
    pub channel: String,
    pub customer_id: Option<Uuid>,
    pub customer_name: Option<String>,
    pub customer_email: Option<String>,
    pub customer_phone: Option<String>,
    pub status: String,
    pub priority: i32,
    pub agent_id: Option<Uuid>,
    pub queue_id: Option<Uuid>,
    pub subject: Option<String>,
    pub initial_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub assigned_at: Option<DateTime<Utc>>,
    pub first_response_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub wait_time_seconds: Option<i32>,
    pub handle_time_seconds: Option<i32>,
    pub satisfaction_rating: Option<i32>,
    pub satisfaction_comment: Option<String>,
    pub tags: Vec<Option<String>>,
    pub metadata: serde_json::Value,
    pub notes: Option<String>,
    pub transfer_count: i32,
    pub created_at: DateTime<Utc>,
    pub branch_id: Option<Uuid>,
}

/// Metadata for a file shared inside an attendant conversation. Serialized
/// into the message's `attachments` JSONB so the thread can render previews
/// and download links without fetching every blob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentMeta {
    pub id: Uuid,
    pub name: String,
    pub content_type: String,
    pub size_bytes: i64,
    /// Download URL served by `GET /api/attendant/attachments/{id}/download`.
    pub url: String,
    /// Optional thumbnail URL for image previews in the thread.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb_url: Option<String>,
}

/// Stored blob backing a conversation attachment (BYTEA row).
#[derive(Debug, Clone, Queryable, Insertable)]
#[diesel(table_name = attendant_attachments)]
pub struct AttendantAttachment {
    pub id: Uuid,
    pub session_id: Uuid,
    pub name: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub data: Vec<u8>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = attendant_session_messages)]
pub struct SessionMessage {
    pub id: Uuid,
    pub session_id: Uuid,
    pub sender_type: String,
    pub sender_id: Option<Uuid>,
    pub sender_name: Option<String>,
    pub content: String,
    pub content_type: String,
    pub attachments: serde_json::Value,
    pub is_internal: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = attendant_queue_agents)]
pub struct QueueAgent {
    pub id: Uuid,
    pub queue_id: Uuid,
    pub agent_id: Uuid,
    pub max_concurrent: i32,
    pub priority: i32,
    pub skills: Vec<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = attendant_agent_status)]
pub struct AgentStatus {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub agent_id: Uuid,
    pub status: String,
    pub status_message: Option<String>,
    pub current_sessions: i32,
    pub max_sessions: i32,
    pub last_activity_at: DateTime<Utc>,
    pub break_started_at: Option<DateTime<Utc>>,
    pub break_reason: Option<String>,
    pub available_since: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub branch_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = attendant_transfers)]
pub struct SessionTransfer {
    pub id: Uuid,
    pub session_id: Uuid,
    pub from_agent_id: Option<Uuid>,
    pub to_agent_id: Option<Uuid>,
    pub to_queue_id: Option<Uuid>,
    pub reason: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = attendant_canned_responses)]
pub struct CannedResponse {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub title: String,
    pub content: String,
    pub shortcut: Option<String>,
    pub category: Option<String>,
    pub queue_id: Option<Uuid>,
    pub is_active: bool,
    pub usage_count: i32,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub branch_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = attendant_tags)]
pub struct AttendantTag {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub branch_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = attendant_wrap_up_codes)]
pub struct WrapUpCode {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub requires_notes: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub branch_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CreateQueueRequest {
    pub name: String,
    pub description: Option<String>,
    pub priority: Option<i32>,
    pub max_wait_minutes: Option<i32>,
    pub auto_assign: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub channel: String,
    pub customer_name: Option<String>,
    pub customer_email: Option<String>,
    pub customer_phone: Option<String>,
    pub customer_id: Option<Uuid>,
    pub queue_id: Option<Uuid>,
    pub subject: Option<String>,
    pub initial_message: Option<String>,
    pub priority: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct AssignSessionRequest {
    pub agent_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct TransferSessionRequest {
    pub to_agent_id: Option<Uuid>,
    pub to_queue_id: Option<Uuid>,
    pub reason: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EndSessionRequest {
    pub wrap_up_code: Option<String>,
    pub notes: Option<String>,
    pub follow_up_required: Option<bool>,
    pub follow_up_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RateSessionRequest {
    pub rating: i32,
    pub comment: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
    pub content_type: Option<String>,
    pub is_internal: Option<bool>,
    pub sender_name: Option<String>,
    /// Attachment metadata for files uploaded via the session attachment API.
    #[serde(default)]
    pub attachments: Option<Vec<AttachmentMeta>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAgentStatusRequest {
    pub status: String,
    pub status_message: Option<String>,
    pub break_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddQueueAgentRequest {
    pub agent_id: Uuid,
    pub max_concurrent: Option<i32>,
    pub priority: Option<i32>,
    pub skills: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCannedResponseRequest {
    pub title: String,
    pub content: String,
    pub shortcut: Option<String>,
    pub category: Option<String>,
    pub queue_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
    pub queue_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub channel: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AttendantStats {
    pub total_sessions_today: i64,
    pub active_sessions: i64,
    pub waiting_sessions: i64,
    pub avg_wait_time_seconds: i64,
    pub avg_handle_time_seconds: i64,
    pub agents_online: i64,
    pub agents_on_break: i64,
    pub satisfaction_avg: f64,
}

#[derive(Debug, Serialize)]
pub struct SessionWithMessages {
    pub session: AttendantSession,
    pub messages: Vec<SessionMessage>,
}

#[derive(Debug, Serialize)]
pub struct QueueWithStats {
    pub queue: AttendantQueue,
    pub waiting_count: i64,
    pub active_count: i64,
    pub agents_count: i64,
}
