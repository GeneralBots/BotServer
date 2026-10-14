pub mod bas_analyzer;
pub mod canvas_api;
pub mod designer_api;
pub mod plugin_manifest;
pub mod ui;
pub mod workflow_canvas;

use diesel::pg::PgConnection;
use std::sync::Arc;
use uuid::Uuid;

pub use botlib::db_pool::DbPool;

pub type GetDefaultBotFn = Arc<dyn Fn(&mut PgConnection) -> (Uuid, String) + Send + Sync>;

pub type GetDesignerErrorContextFn = Arc<dyn Fn(&str) -> Option<String> + Send + Sync>;

pub type GetContentTypeFn = Arc<dyn Fn(&str) -> &'static str + Send + Sync>;

pub type GetStackPathFn = Arc<dyn Fn() -> String + Send + Sync>;

pub type LoadFromDriveFn = Arc<dyn Fn(&str, &str) -> Result<String, String> + Send + Sync>;

pub type WriteToDriveFn = Arc<dyn Fn(&str, &str, &[u8], &str) -> Result<(), String> + Send + Sync>;

pub type CallLlmFn = Arc<dyn Fn(&str, &serde_json::Value) -> Result<String, String> + Send + Sync>;

pub type GetConfigFn = Arc<dyn Fn(&str, &str, Option<&str>) -> Result<String, String> + Send + Sync>;

pub struct DesignerState {
    pub conn: Arc<DbPool>,
    pub get_default_bot: GetDefaultBotFn,
    pub get_designer_error_context: GetDesignerErrorContextFn,
    pub get_content_type: GetContentTypeFn,
    pub get_stack_path: GetStackPathFn,
    pub load_from_drive: LoadFromDriveFn,
    pub write_to_drive: WriteToDriveFn,
    pub call_llm: CallLlmFn,
    pub get_config: GetConfigFn,
    pub bucket_name: String,
    pub site_path: Option<String>,
}
