use diesel::PgConnection;
use std::sync::Arc;
use uuid::Uuid;

pub use botlib::db_pool::DbPool;

pub struct ChannelState {
    pub conn: Arc<DbPool>,
    pub get_default_bot: GetDefaultBotFn,
    pub get_config: GetConfigFn,
    pub stream_response: StreamResponseFn,
    pub attendant_broadcast: Option<tokio::sync::broadcast::Sender<AttendantNotification>>,
}

pub type GetDefaultBotFn = Arc<dyn Fn(&mut PgConnection) -> (Uuid, String) + Send + Sync>;
pub type GetConfigFn = Arc<dyn Fn(&Uuid, &str, Option<&str>) -> Result<String, String> + Send + Sync>;
pub type StreamResponseFn = Arc<
    dyn Fn(
        botlib::models::UserMessage,
        tokio::sync::mpsc::Sender<botlib::models::BotResponse>,
    ) -> tokio::task::JoinHandle<Result<(), String>>
    + Send
    + Sync,
>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AttendantNotification {
    #[serde(rename = "type")]
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
