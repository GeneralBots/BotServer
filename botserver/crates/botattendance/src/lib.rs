pub mod drive;
pub mod keyword_services;
pub mod llm_types;
pub mod llm_parser;
pub mod models;
pub mod queue_types;
pub mod queue_csv;
pub mod queue_handlers;
pub mod routes;
pub mod schema;
pub mod sla;
pub mod webhooks;
#[cfg(feature = "llm")]
pub mod llm_assist_types;
#[cfg(feature = "llm")]
pub mod llm_assist_config;
#[cfg(feature = "llm")]
pub mod llm_assist_helpers;
#[cfg(feature = "llm")]
pub mod llm_assist_handlers;
#[cfg(feature = "llm")]
pub mod llm_assist_commands;

use std::sync::Arc;

pub use botlib::db_pool::DbPool;

pub type GetDefaultBotFn = Arc<dyn Fn(&mut diesel::PgConnection) -> uuid::Uuid + Send + Sync>;

pub type LlmGenerateFn = Arc<dyn Fn(&str, &serde_json::Value, &str, &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> + Send + Sync>;

pub type ProcessContentFn = Arc<dyn Fn(&str, &str) -> String + Send + Sync>;

pub type ConfigGetFn = Arc<dyn Fn(&uuid::Uuid, &str) -> String + Send + Sync>;

pub type SendBotResponseFn = Arc<dyn Fn(models::BotResponse) -> Result<(), Box<dyn std::error::Error + Send + Sync>> + Send + Sync>;

pub type BroadcastNotificationFn = Arc<dyn Fn(models::AttendantNotification) + Send + Sync>;

pub type SaveMessageFn = Arc<dyn Fn(uuid::Uuid, &str, &str, i32) -> Result<(), String> + Send + Sync>;

pub struct AttendanceConfig {
    pub pool: Arc<DbPool>,
    pub master_key: Vec<u8>,
    pub get_default_bot: GetDefaultBotFn,
    pub llm_generate: LlmGenerateFn,
    pub process_content: ProcessContentFn,
    pub config_get: ConfigGetFn,
    pub send_bot_response: Option<SendBotResponseFn>,
    pub broadcast_notification: Option<BroadcastNotificationFn>,
    pub save_message: Option<SaveMessageFn>,
}

pub use drive::{AttendanceDriveConfig, AttendanceDriveService, RecordMetadata, SyncResult};
pub use keyword_services::{
    AttendanceCommand, AttendanceRecord, AttendanceResponse, AttendanceService, KeywordConfig,
    KeywordParser, ParsedCommand,
};
pub use queue_types::{
    AssignRequest, AttendantStats, AttendantStatus, QueueFilters, QueueItem, QueueStatus,
    TransferRequest,
};
#[cfg(feature = "llm")]
pub use llm_assist_types::*;
#[cfg(feature = "llm")]
pub use llm_parser::{
    AttendantTip as ParsedTip, SmartReply as ParsedSmartReply,
    ConversationSummary as ParsedConversationSummary, SentimentAnalysis as ParsedSentimentAnalysis,
    parse_tips_response as parse_tips_response_alt,
    parse_polish_response as parse_polish_response_alt,
    parse_smart_replies_response as parse_smart_replies_response_alt,
    parse_summary_response as parse_summary_response_alt,
    parse_sentiment_response as parse_sentiment_response_alt,
    extract_json as extract_json_alt,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attendance_config_type_compatibility() {
        fn _assert_send_sync<T: Send + Sync>() {}
        _assert_send_sync::<AttendanceConfig>();
    }
}
