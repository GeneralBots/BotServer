//! `types::core` — split per #1443 (AGENTS.md 450-line rule).

use super::*;

pub use botlib::db_pool::DbPool;

pub(crate) type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub(crate) type BoxFuture<T> = std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, BoxError>> + Send>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VibeUseCase {
    SoftwareDevelopment,
    CustomerSupport,
    FinancialAnalysis,
}

impl std::fmt::Display for VibeUseCase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SoftwareDevelopment => write!(f, "software_development"),
            Self::CustomerSupport => write!(f, "customer_support"),
            Self::FinancialAnalysis => write!(f, "financial_analysis"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMessage {
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VibeToolResult {
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
    pub latency_ms: u64,
}

/// LLM provider settings resolved for a specific bot (Issue #795).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmConfig {
    pub model: String,
    pub key: String,
    pub url: String,
}

/// Vibe's seam over the single provider contract
/// (`botlib::traits::LLMProvider`, #1498): it adds only the system-prompt
/// argument, so implementations forward to the canonical trait.
pub trait VibeLlmOps: Send + Sync {
    fn generate(
        &self,
        prompt: &str,
        system_prompt: Option<&str>,
        config: &serde_json::Value,
        model: &str,
        key: &str,
    ) -> BoxFuture<String>;
}

pub trait VibeConfigOps: Send + Sync {
    fn get_config(
        &self,
        bot_id: &Uuid,
        key: &str,
        default: Option<&str>,
    ) -> Result<String, BoxError>;

    fn set_config(
        &self,
        bot_id: &Uuid,
        key: &str,
        value: &str,
    ) -> Result<(), BoxError>;
}
