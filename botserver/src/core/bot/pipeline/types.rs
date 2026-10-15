use std::fmt;
use std::sync::Arc;

use botcore::shared::state::AppState;
use uuid::Uuid;

use super::sink::ChannelSink;

/// Identity of the bot session a pipeline stage runs for.
///
/// Every stage needs the same set of values; bundling them keeps the stage
/// signatures inside the argument budget (`clippy::too_many_arguments`) and
/// makes what a stage depends on explicit at the call site.
#[derive(Clone, Copy)]
pub struct SessionIdentity<'a> {
    pub sink: &'a dyn ChannelSink,
    pub state: &'a Arc<AppState>,
    pub bot_uuid: Uuid,
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub bot_name: &'a str,
}

/// The LLM a caller resolved for the current turn.
#[derive(Clone, Copy)]
pub struct LlmTarget<'a> {
    pub provider: Option<&'a Arc<dyn botlib::traits::LLMProvider>>,
    pub model: &'a str,
    pub key: &'a str,
}

#[derive(Debug, Clone)]
pub enum PipelineError {
    Config(String),
    Session(String),
    History(String),
    KnowledgeBase(String),
    ToolExecution(String),
    LLM(String),
    Transport(String),
    StartBas(String),
    RateLimit(String),
    AccessDenied(String),
    BotNotFound(String),
}

impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(msg) => write!(f, "Config error: {msg}"),
            Self::Session(msg) => write!(f, "Session error: {msg}"),
            Self::History(msg) => write!(f, "History error: {msg}"),
            Self::KnowledgeBase(msg) => write!(f, "KB error: {msg}"),
            Self::ToolExecution(msg) => write!(f, "Tool exec error: {msg}"),
            Self::LLM(msg) => write!(f, "LLM error: {msg}"),
            Self::Transport(msg) => write!(f, "Transport error: {msg}"),
            Self::StartBas(msg) => write!(f, "start.bas error: {msg}"),
            Self::RateLimit(msg) => write!(f, "Rate limit: {msg}"),
            Self::AccessDenied(msg) => write!(f, "Access denied: {msg}"),
            Self::BotNotFound(msg) => write!(f, "Bot not found: {msg}"),
        }
    }
}

impl std::error::Error for PipelineError {}

pub type PipelineResult<T> = Result<T, PipelineError>;

impl From<String> for PipelineError {
    fn from(msg: String) -> Self {
        Self::Session(msg)
    }
}