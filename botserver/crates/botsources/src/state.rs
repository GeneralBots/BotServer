pub use diesel::r2d2::Pool;
pub use diesel::r2d2::ConnectionManager;
pub use diesel::PgConnection;

pub use botlib::db_pool::DbPool;

use std::sync::Arc;

pub type GetDefaultBotFn = Arc<dyn Fn(&mut PgConnection) -> (uuid::Uuid, String) + Send + Sync>;

pub type GetWorkPathFn = Arc<dyn Fn() -> String + Send + Sync>;

pub type GetKeywordsFn = Arc<dyn Fn() -> Vec<String> + Send + Sync>;

pub type McpCsvLoaderFn = Arc<dyn Fn(&str, &str) -> Box<dyn McpCsvLoaderOps> + Send + Sync>;

pub trait McpCsvLoaderOps: Send + Sync {
    fn load(&self) -> McpLoadResult;
    fn load_server(&self, name: &str) -> Option<McpServerInfo>;
    fn add_server(&self, row: &McpCsvRowData) -> Result<(), String>;
    fn remove_server(&self, name: &str) -> Result<bool, String>;
    fn csv_exists(&self) -> bool;
}

#[derive(Debug, Clone)]
pub struct McpLoadResult {
    pub file_path: std::path::PathBuf,
    pub servers: Vec<McpServerInfo>,
    pub lines_processed: usize,
    pub errors: Vec<McpLoadError>,
}

#[derive(Debug, Clone, Default)]
pub struct McpLoadError {
    pub line: usize,
    pub message: String,
    pub recoverable: bool,
}

#[derive(Debug, Clone)]
pub struct McpServerInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub server_type: String,
    pub status: String,
    pub enabled: bool,
    pub tools: Vec<McpToolInfo>,
}

#[derive(Debug, Clone)]
pub struct McpToolInfo {
    pub name: String,
    pub description: String,
    pub risk_level: String,
    pub requires_approval: bool,
}

#[derive(Debug, Clone)]
pub struct McpCsvRowData {
    pub name: String,
    pub connection_type: String,
    pub command: String,
    pub args: String,
    pub description: String,
    pub enabled: bool,
    pub auth_type: Option<String>,
    pub auth_env: Option<String>,
    pub risk_level: Option<String>,
    pub requires_approval: bool,
}

pub trait ConfigManagerOps: Send + Sync {
    fn get_config(&self, bot_id: &uuid::Uuid, key: &str, default: Option<&str>) -> Result<String, Box<dyn std::error::Error>>;
    fn set_config(&self, bot_id: &uuid::Uuid, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>>;
}

pub struct AppState {
    pub conn: DbPool,
    pub config_manager: Arc<dyn ConfigManagerOps>,
    pub get_default_bot: Option<GetDefaultBotFn>,
    pub get_work_path: Option<GetWorkPathFn>,
    pub get_keywords: Option<GetKeywordsFn>,
    pub mcp_loader: Option<McpCsvLoaderFn>,
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            conn: self.conn.clone(),
            config_manager: Arc::clone(&self.config_manager),
            get_default_bot: self.get_default_bot.clone(),
            get_work_path: self.get_work_path.clone(),
            get_keywords: self.get_keywords.clone(),
            mcp_loader: self.mcp_loader.clone(),
        }
    }
}

pub fn get_work_path_or_default(get_work_path: &Option<GetWorkPathFn>) -> String {
    match get_work_path {
        Some(f) => f(),
        None => "/opt/gbo/work".to_string(),
    }
}

pub fn get_keywords_or_default(get_keywords: &Option<GetKeywordsFn>) -> Vec<String> {
    match get_keywords {
        Some(f) => f(),
        None => Vec::new(),
    }
}

pub fn make_mcp_loader(mcp_loader: &Option<McpCsvLoaderFn>, work_path: &str, bot_id: &str) -> Box<dyn McpCsvLoaderOps> {
    match mcp_loader {
        Some(f) => f(work_path, bot_id),
        None => Box::new(NoOpMcpLoader),
    }
}

struct NoOpMcpLoader;

impl McpCsvLoaderOps for NoOpMcpLoader {
    fn load(&self) -> McpLoadResult {
        McpLoadResult {
            file_path: std::path::PathBuf::new(),
            servers: Vec::new(),
            lines_processed: 0,
            errors: Vec::new(),
        }
    }
    fn load_server(&self, _name: &str) -> Option<McpServerInfo> { None }
    fn add_server(&self, _row: &McpCsvRowData) -> Result<(), String> { Err("MCP loader not configured".to_string()) }
    fn remove_server(&self, _name: &str) -> Result<bool, String> { Err("MCP loader not configured".to_string()) }
    fn csv_exists(&self) -> bool { false }
}
