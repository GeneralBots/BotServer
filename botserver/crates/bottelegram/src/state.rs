use diesel::PgConnection;
use std::sync::Arc;
use uuid::Uuid;

pub use botlib::db_pool::DbPool;

pub struct ChannelState {
    pub conn: Arc<DbPool>,
    pub get_default_bot: GetDefaultBotFn,
    pub get_config: GetConfigFn,
    pub put_media: PutMediaFn,
    pub stream_response: StreamResponseFn,
    pub attendant_broadcast: Option<tokio::sync::broadcast::Sender<AttendantNotification>>,
}

pub type GetDefaultBotFn = Arc<dyn Fn(&mut PgConnection) -> (Uuid, String) + Send + Sync>;
pub type GetConfigFn = Arc<dyn Fn(&Uuid, &str, Option<&str>) -> Result<String, String> + Send + Sync>;

/// Writes an inbound media file into the bot's Drive and returns the stored
/// path relative to `{bot}.gbdrive` (for example `inbox/a1b2.jpg`).
pub type PutMediaFn = Arc<
    dyn Fn(
            Uuid,
            String,
            Vec<u8>,
            Option<String>,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send>>
        + Send
        + Sync,
>;
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
