//! Shared runtime state: notifications, progress events, extensions and AppState.

pub(crate) use crate::config::AppConfig;
pub(crate) use botlib::models::BotResponse;
pub(crate) use crate::shared::utils::DbPool;
#[cfg(feature = "cache")]
pub(crate) use redis::Client as RedisClient;
pub(crate) use std::any::Any;
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::{broadcast, mpsc, RwLock};

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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentActivity {
    pub phase: String,
    pub items_processed: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items_total: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_per_min: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta_seconds: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_item: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_processed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_created: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tables_created: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_lines: Option<Vec<String>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskProgressEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub task_id: String,
    pub step: String,
    pub message: String,
    pub progress: u8,
    pub total_steps: u8,
    pub current_step: u8,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activity: Option<AgentActivity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Clone, Default)]
pub struct Extensions {
    map: Arc<RwLock<HashMap<std::any::TypeId, Arc<dyn Any + Send + Sync>>>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BillingAlertNotification {
    pub alert_id: uuid::Uuid,
    pub organization_id: uuid::Uuid,
    pub severity: String,
    pub alert_type: String,
    pub title: String,
    pub message: String,
    pub metric: String,
    pub percentage: f64,
    pub triggered_at: chrono::DateTime<chrono::Utc>,
}

/// Stub type for modules not yet extracted into botcore.
/// Replaced by concrete types in botserver via extensions or downcasting.
pub type UnresolvedService = Arc<dyn Any + Send + Sync>;

pub struct AppState {
    pub drive: Option<Arc<dyn botlib::traits::DriveRepository>>,
    #[cfg(feature = "cache")]
    pub cache: Option<Arc<RedisClient>>,
    pub bucket_name: String,
    pub config: Option<AppConfig>,
    pub conn: DbPool,
    pub database_url: String,
    pub bot_database_manager: Arc<dyn botlib::traits::BotDatabaseService>,
    pub session_manager: Arc<tokio::sync::Mutex<dyn botlib::traits::SessionManagerService>>,
    pub metrics_collector: crate::shared::analytics::MetricsCollector,
    pub channels: Arc<tokio::sync::Mutex<HashMap<String, Arc<dyn botlib::traits::ChannelAdapter>>>>,
    pub response_channels: Arc<tokio::sync::Mutex<HashMap<String, mpsc::Sender<BotResponse>>>>,
    pub active_streams: Arc<tokio::sync::Mutex<HashMap<String, broadcast::Sender<()>>>>,
    pub hear_channels: Arc<std::sync::Mutex<HashMap<uuid::Uuid, std::sync::mpsc::SyncSender<String>>>>,
    pub web_adapter: Arc<dyn botlib::traits::ChannelAdapter>,
    pub voice_adapter: Arc<dyn botlib::traits::ChannelAdapter>,
    pub kb_manager: Option<Arc<dyn botlib::traits::KnowledgeBase>>,
    pub script_runner: Option<Arc<dyn botlib::traits::ScriptRunner>>,
    pub task_engine: Option<Arc<dyn botlib::traits::TaskEngineService>>,
    pub task_scheduler: Option<Arc<dyn botlib::traits::TaskSchedulerService>>,
    pub extensions: Extensions,
    pub attendant_broadcast: Option<broadcast::Sender<AttendantNotification>>,
    pub task_progress_broadcast: Option<broadcast::Sender<TaskProgressEvent>>,
    pub billing_alert_broadcast: Option<broadcast::Sender<BillingAlertNotification>>,
    pub task_manifests: Arc<std::sync::RwLock<HashMap<String, Arc<dyn botlib::traits::TaskOrchestrator>>>>,
    pub terminal_manager: Option<UnresolvedService>,
    pub project_service: Option<UnresolvedService>,
    pub legal_service: Option<UnresolvedService>,
    pub jwt_manager: Option<Arc<dyn botlib::traits::JwtService>>,
    pub auth_provider_registry: Option<UnresolvedService>,
    pub rbac_manager: Option<Arc<dyn botlib::traits::RbacService>>,
    pub auth_service: Option<Arc<tokio::sync::Mutex<dyn botlib::traits::AuthServiceTrait>>>,
    pub llm_provider: Option<Arc<dyn botlib::traits::LLMProvider>>,
    pub dynamic_llm_provider: Option<UnresolvedService>,
    pub start_bas_guards: Arc<tokio::sync::Mutex<HashMap<uuid::Uuid, bool>>>,
    pub pending_stream_responses: Arc<tokio::sync::Mutex<HashMap<String, String>>>,
}

mod agent_activity;
mod task_progress;
mod extensions;
mod app_state;
