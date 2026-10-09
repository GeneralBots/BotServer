pub(crate) use chrono::{DateTime, Utc};
pub(crate) use reqwest::Client;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwilioConfig {
    pub account_sid: String,
    pub auth_token: String,
    pub from_number: String,
    pub messaging_service_sid: Option<String>,
    pub status_callback_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsMessage {
    pub sid: String,
    pub account_sid: String,
    pub from: String,
    pub to: String,
    pub body: String,
    pub status: MessageStatus,
    pub direction: MessageDirection,
    pub date_created: Option<DateTime<Utc>>,
    pub date_sent: Option<DateTime<Utc>>,
    pub date_updated: Option<DateTime<Utc>>,
    pub price: Option<String>,
    pub price_unit: Option<String>,
    pub error_code: Option<i32>,
    pub error_message: Option<String>,
    pub num_segments: Option<u32>,
    pub num_media: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageStatus {
    Queued,
    Sending,
    Sent,
    Delivered,
    Undelivered,
    Failed,
    Receiving,
    Received,
    Accepted,
    Scheduled,
    Read,
    PartiallyDelivered,
    Canceled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MessageDirection {
    Inbound,
    OutboundApi,
    OutboundCall,
    OutboundReply,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendSmsRequest {
    pub to: String,
    pub body: String,
    pub media_url: Option<Vec<String>>,
    pub status_callback: Option<String>,
    pub max_price: Option<String>,
    pub validity_period: Option<u32>,
    pub schedule_type: Option<ScheduleType>,
    pub send_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ScheduleType {
    Fixed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingWebhook {
    pub message_sid: String,
    pub account_sid: String,
    pub from: String,
    pub to: String,
    pub body: String,
    pub num_media: Option<u32>,
    pub num_segments: Option<u32>,
    pub sms_sid: Option<String>,
    pub sms_status: Option<String>,
    pub api_version: Option<String>,
    pub from_city: Option<String>,
    pub from_state: Option<String>,
    pub from_zip: Option<String>,
    pub from_country: Option<String>,
    pub to_city: Option<String>,
    pub to_state: Option<String>,
    pub to_zip: Option<String>,
    pub to_country: Option<String>,
    pub media_urls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusCallback {
    pub message_sid: String,
    pub message_status: MessageStatus,
    pub account_sid: String,
    pub from: String,
    pub to: String,
    pub api_version: Option<String>,
    pub error_code: Option<i32>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwilioPhoneNumber {
    pub sid: String,
    pub phone_number: String,
    pub friendly_name: String,
    pub capabilities: PhoneCapabilities,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneCapabilities {
    pub sms: bool,
    pub mms: bool,
    pub voice: bool,
    pub fax: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    pub id: Uuid,
    pub phone_number: String,
    pub messages: Vec<ConversationMessage>,
    pub created_at: DateTime<Utc>,
    pub last_message_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub sid: String,
    pub direction: MessageDirection,
    pub body: String,
    pub timestamp: DateTime<Utc>,
    pub status: MessageStatus,
}

pub struct TwilioSmsChannel {
    config: TwilioConfig,
    http_client: Client,
    conversations: Arc<RwLock<HashMap<String, ConversationContext>>>,
}

    /// Twilio API base URL is a compile-time constant: credentials are scoped
    /// to this host and must never be sent to a runtime-configured one.
    const API_BASE: &str = "https://api.twilio.com/2010-04-01";

#[derive(Debug, Deserialize)]
struct TwilioMessageResponse {
    sid: String,
    account_sid: String,
    from: Option<String>,
    to: String,
    body: Option<String>,
    status: String,
    direction: Option<String>,
    date_created: Option<String>,
    date_sent: Option<String>,
    date_updated: Option<String>,
    price: Option<String>,
    price_unit: Option<String>,
    error_code: Option<i32>,
    error_message: Option<String>,
    num_segments: Option<String>,
    num_media: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageListResponse {
    messages: Vec<TwilioMessageResponse>,
}

#[derive(Debug, Deserialize)]
struct PhoneNumberListResponse {
    incoming_phone_numbers: Vec<PhoneNumberResponse>,
}

#[derive(Debug, Deserialize)]
struct PhoneNumberResponse {
    sid: String,
    phone_number: String,
    friendly_name: String,
    capabilities: CapabilitiesResponse,
    status: String,
}

#[derive(Debug, Deserialize)]
struct CapabilitiesResponse {
    sms: Option<bool>,
    mms: Option<bool>,
    voice: Option<bool>,
    fax: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwilioApiError {
    pub code: i32,
    pub message: String,
    pub more_info: Option<String>,
    pub status: i32,
}

#[derive(Debug, Clone)]
pub enum TwilioError {
    NetworkError(String),
    ApiError(TwilioApiError),
    ParseError(String),
    ConfigError(String),
    InvalidSignature,
}

mod part1;
mod part2;
mod part3;
mod part4;

use part3::*;
pub use part4::*;
