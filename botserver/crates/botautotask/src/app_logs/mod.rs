pub(crate) use chrono::{DateTime, Duration, Utc};
pub(crate) use log::{debug, error, info, warn};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::{HashMap, VecDeque};
pub(crate) use std::sync::{Arc, LazyLock, RwLock};
pub(crate) use uuid::Uuid;

const MAX_LOGS_PER_APP: usize = 500;
const MAX_LOGS_FOR_DESIGNER: usize = 50;
const LOG_RETENTION_DAYS: i64 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppLogEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub source: LogSource,
    pub app_name: String,
    pub bot_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub message: String,
    pub details: Option<String>,
    pub file_path: Option<String>,
    pub line_number: Option<u32>,
    pub stack_trace: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogSource {
    Server,
    Client,
    Generator,
    Designer,
    Validation,
    Runtime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientLogRequest {
    pub app_name: String,
    pub level: String,
    pub message: String,
    pub details: Option<String>,
    pub file_path: Option<String>,
    pub line_number: Option<u32>,
    pub stack_trace: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogQueryParams {
    pub app_name: Option<String>,
    pub level: Option<String>,
    pub source: Option<String>,
    pub limit: Option<usize>,
    pub since: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogStats {
    pub total_logs: usize,
    pub errors: usize,
    pub warnings: usize,
    pub by_app: HashMap<String, usize>,
}

pub struct AppLogStore {
    logs: RwLock<HashMap<String, VecDeque<AppLogEntry>>>,
    global_logs: RwLock<VecDeque<AppLogEntry>>,
}

pub static APP_LOGS: LazyLock<Arc<AppLogStore>> = LazyLock::new(|| Arc::new(AppLogStore::new()));

mod part1;
mod part2;
mod part3;
mod part4;

pub use part4::*;
