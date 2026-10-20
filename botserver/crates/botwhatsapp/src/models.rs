use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::message_history;

#[derive(Debug, Clone, Queryable, Serialize, Deserialize)]
#[diesel(table_name = bots)]
pub struct Bot {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Queryable, Serialize, Deserialize)]
pub struct BotConfiguration {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = message_history)]
pub struct NewMessage {
    pub id: Uuid,
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub role: i32,
    pub content_encrypted: String,
    pub message_type: i32,
    pub media_url: Option<String>,
    pub token_count: i32,
    pub processing_time_ms: Option<i32>,
    pub llm_model: Option<String>,
    pub created_at: DateTime<Utc>,
    pub message_index: i32,
}

#[derive(Debug, Clone, Queryable, Serialize, Deserialize)]
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
    pub branch_id: Option<Uuid>,
    pub session_id: Option<String>,
    pub data: Option<serde_json::Value>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Queryable, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppWebhookPayload {
    pub object: Option<String>,
    pub entry: Vec<WebhookEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEntry {
    pub id: Option<String>,
    pub changes: Vec<WebhookChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookChange {
    pub value: WebhookValue,
    pub field: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookValue {
    pub messaging_product: Option<String>,
    pub metadata: Option<WebhookMetadata>,
    pub messages: Option<Vec<WhatsAppMessage>>,
    pub statuses: Option<Vec<WhatsAppStatus>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookMetadata {
    pub display_phone_number: Option<String>,
    pub phone_number_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppMessage {
    pub from: Option<String>,
    pub id: Option<String>,
    pub timestamp: Option<String>,
    #[serde(rename = "type")]
    pub message_type: Option<String>,
    pub text: Option<TextContent>,
    pub image: Option<ImageContent>,
    pub video: Option<VideoContent>,
    pub audio: Option<AudioContent>,
    pub document: Option<DocumentContent>,
    pub sticker: Option<StickerContent>,
    pub interactive: Option<InteractiveContent>,
    pub button: Option<ButtonContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent {
    pub id: Option<String>,
    pub mime_type: Option<String>,
    pub caption: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoContent {
    pub id: Option<String>,
    pub mime_type: Option<String>,
    pub filename: Option<String>,
    pub caption: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioContent {
    pub id: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentContent {
    pub id: Option<String>,
    pub mime_type: Option<String>,
    pub filename: Option<String>,
    pub caption: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickerContent {
    pub id: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveContent {
    #[serde(rename = "type")]
    pub interactive_type: Option<String>,
    pub button_reply: Option<ButtonReplyContent>,
    pub list_reply: Option<ListReplyContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonContent {
    pub payload: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonReplyContent {
    pub id: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListReplyContent {
    pub id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppStatus {
    pub id: Option<String>,
    pub status: Option<String>,
    pub timestamp: Option<String>,
    pub recipient_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub to: String,
    pub message: String,
    pub bot_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttendantNotification {
    pub bot_id: Uuid,
    pub phone_number: String,
    pub message: String,
    pub session_id: Option<Uuid>,
}
