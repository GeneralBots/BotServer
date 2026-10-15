use crate::models::UserSession;
use crate::BotResult;
use std::sync::Arc;
use uuid::Uuid;

pub use botlib::db_pool::DbPool;

pub trait SessionManager: Send + Sync {
    fn get_session_by_id(&self, session_id: Uuid) -> BotResult<Option<UserSession>>;
    fn get_or_create_user_session(
        &self,
        user_id: Uuid,
        bot_id: Uuid,
        title: &str,
    ) -> BotResult<Option<UserSession>>;
    fn create_session(
        &self,
        user_id: Uuid,
        bot_id: Uuid,
        title: &str,
    ) -> BotResult<UserSession>;
    fn save_message(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        role: i32,
        content: &str,
        message_type: i32,
    ) -> BotResult<()>;
    fn get_conversation_history(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        limit: Option<i64>,
    ) -> BotResult<Vec<(String, String)>>;
    fn get_session_context_data(
        &self,
        session_id: &Uuid,
        user_id: &Uuid,
    ) -> BotResult<serde_json::Value>;
    fn update_session_context(
        &self,
        session_id: &Uuid,
        user_id: &Uuid,
        context_json: String,
    ) -> BotResult<()>;
    fn get_user_sessions(&self, user_id: Uuid) -> BotResult<Vec<UserSession>>;
}

pub trait ConfigManager: Send + Sync {
    fn get_config(&self, bot_id: &Uuid, key: &str, default: Option<&str>) -> BotResult<String>;
    fn get_bot_config_value(&self, bot_id: &Uuid, key: &str) -> BotResult<String>;
}

pub trait ScriptRunner: Send + Sync {
    fn run_script(
        &self,
        db_pool: &DbPool,
        session: &UserSession,
        bot_id: Uuid,
        ast_content: &str,
    ) -> BotResult<String>;
    fn set_variable(&self, key: &str, value: &str) -> BotResult<()>;
}

pub trait SuggestionProvider: Send + Sync {
    fn get_suggestions(
        &self,
        bot_id: &str,
        session_id: &str,
    ) -> Vec<botlib::Suggestion>;
    fn get_switchers(
        &self,
        bot_id: &str,
        session_id: &str,
    ) -> Vec<botlib::Switcher>;
    fn resolve_active_switchers(
        &self,
        bot_id: &str,
        session_id: &str,
        active_switchers: &[String],
    ) -> String;
}

pub trait KbSearchProvider: Send + Sync {
    fn search(
        &self,
        bot_id: Uuid,
        bot_name: &str,
        kb_name: &str,
        query: &str,
        max_results: usize,
    ) -> BotResult<Vec<KbSearchResult>>;
}

#[derive(Debug, Clone)]
pub struct KbSearchResult {
    pub content: String,
    pub document_path: String,
    pub score: f32,
}

pub trait CommandExecutor: Send + Sync {
    fn execute(&self, program: &str, args: &[&str]) -> BotResult<std::process::Output>;
}

pub trait WebAdapter: Send + Sync {
    fn add_connection(&self, session_id: String, tx: tokio::sync::mpsc::Sender<serde_json::Value>);
    fn remove_connection(&self, session_id: &str);
}

pub trait DriveMonitor: Send + Sync {
    fn is_bot_mounted(&self, bot_name: &str) -> bool;
}

pub trait CacheProvider: Send + Sync {
    fn get_connection(&self) -> BotResult<Arc<dyn redis::aio::ConnectionLike>>;
}

pub trait SystemMetricsProvider: Send + Sync {
    fn get_system_metrics(&self) -> BotResult<SystemMetrics>;
}

#[derive(Debug, Clone, Default)]
pub struct SystemMetrics {
    pub gpu_usage: Option<f32>,
    pub cpu_usage: f32,
}

pub trait SecretsManager: Send + Sync {
    fn get_vectordb_config(&self) -> (String, Option<String>);
}
