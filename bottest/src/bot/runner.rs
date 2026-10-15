use super::{BotResponse, ConversationState, ResponseContentType};
use crate::fixtures::{Bot, Channel, Customer, Session};
use crate::harness::TestContext;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct BotRunnerConfig {
    pub working_dir: PathBuf,
    pub timeout: Duration,
    pub use_mocks: bool,
    pub env_vars: HashMap<String, String>,
    pub capture_logs: bool,
    pub log_level: LogLevel,
}

impl Default for BotRunnerConfig {
    fn default() -> Self {
        Self {
            working_dir: std::env::temp_dir().join("bottest"),
            timeout: Duration::from_secs(30),
            use_mocks: true,
            env_vars: HashMap::new(),
            capture_logs: true,
            log_level: LogLevel::Info,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

pub struct BotRunner {
    config: BotRunnerConfig,
    bot: Option<Bot>,
    sessions: Arc<Mutex<HashMap<Uuid, SessionState>>>,
    script_cache: Arc<Mutex<HashMap<String, String>>>,
    metrics: Arc<Mutex<RunnerMetrics>>,
}

pub(crate) struct SessionState {
    session: Session,
    customer: Customer,
    channel: Channel,
    context: HashMap<String, serde_json::Value>,
    conversation_state: ConversationState,
    message_count: usize,
    started_at: Instant,
}

#[derive(Debug, Default, Clone)]
pub struct RunnerMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_latency_ms: u64,
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
    pub script_executions: u64,
    pub transfer_to_human_count: u64,
}

impl RunnerMetrics {
    pub const fn avg_latency_ms(&self) -> u64 {
        match self.total_requests {
            0 => 0,
            n => self.total_latency_ms / n,
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_requests > 0 {
            (self.successful_requests as f64 / self.total_requests as f64) * 100.0
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub session_id: Uuid,
    pub response: Option<BotResponse>,
    pub state: ConversationState,
    pub execution_time: Duration,
    pub logs: Vec<LogEntry>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: LogLevel,
    pub message: String,
    pub context: HashMap<String, String>,
}

mod part2;
mod part3;
pub use part3::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bot_runner_config_default() {
        let config = BotRunnerConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert!(config.use_mocks);
        assert!(config.capture_logs);
    }

    #[test]
    fn test_runner_metrics_avg_latency() {
        let metrics = RunnerMetrics {
            total_requests: 10,
            total_latency_ms: 1000,
            ..Default::default()
        };

        assert_eq!(metrics.avg_latency_ms(), 100);
    }

    #[test]
    fn test_runner_metrics_success_rate() {
        let metrics = RunnerMetrics {
            total_requests: 100,
            successful_requests: 95,
            ..Default::default()
        };

        assert!((metrics.success_rate() - 95.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_runner_metrics_zero_requests() {
        let metrics = RunnerMetrics::default();
        assert_eq!(metrics.avg_latency_ms(), 0);
        assert!(metrics.success_rate().abs() < f64::EPSILON);
    }

    #[test]
    fn test_bot_runner_new() {
        let runner = BotRunner::new();
        assert_eq!(runner.active_session_count(), 0);
    }

    #[test]
    fn test_load_script() {
        let runner = BotRunner::new();
        runner.load_script("test", "TALK \"Hello\"");

        let cache = runner.script_cache.lock().unwrap();
        assert!(cache.contains_key("test"));
        drop(cache);
    }

    #[test]
    fn test_start_session() {
        let runner = BotRunner::new();
        let customer = Customer::default();

        let session_id = runner.start_session(customer).unwrap();

        assert_eq!(runner.active_session_count(), 1);
        assert!(runner.get_session_info(session_id).is_some());
    }

    #[test]
    fn test_end_session() {
        let runner = BotRunner::new();
        let customer = Customer::default();

        let session_id = runner.start_session(customer).unwrap();
        assert_eq!(runner.active_session_count(), 1);

        runner.end_session(session_id).unwrap();
        assert_eq!(runner.active_session_count(), 0);
    }

    #[tokio::test]
    async fn test_process_message() {
        let mut runner = BotRunner::new();
        let bot = Bot::default();
        runner.set_bot(bot);
        let customer = Customer::default();

        let session_id = runner.start_session(customer).unwrap();
        let result = runner.process_message(session_id, "Hello").await.unwrap();

        assert!(result.response.is_some());
        assert!(result.error.is_none());
        assert_eq!(result.state, ConversationState::WaitingForUser);
    }

    #[tokio::test]
    async fn test_process_message_invalid_session() {
        let runner = BotRunner::new();
        let invalid_session_id = Uuid::new_v4();

        let result = runner
            .process_message(invalid_session_id, "Hello")
            .await
            .unwrap();

        assert!(result.response.is_none());
        assert!(result.error.is_some());
        assert_eq!(result.state, ConversationState::Error);
    }

    #[test]
    fn test_execute_script() {
        let runner = BotRunner::new();
        runner.load_script("greeting", "TALK \"Hello\"");

        let result = runner.execute_script("greeting", "Hi").unwrap();

        assert!(result.response.is_some());
        assert!(result.error.is_none());
    }

    #[test]
    fn test_execute_script_not_found() {
        let runner = BotRunner::new();

        let result = runner.execute_script("nonexistent", "Hi").unwrap();

        assert!(result.response.is_none());
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("not found"));
    }

    #[test]
    fn test_metrics_tracking() {
        let runner = BotRunner::new();
        let metrics = runner.metrics();

        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.successful_requests, 0);
    }

    #[test]
    fn test_reset_metrics() {
        let runner = BotRunner::new();

        {
            let mut metrics = runner.metrics.lock().unwrap();
            metrics.total_requests = 100;
        }

        runner.reset_metrics();
        let metrics = runner.metrics();

        assert_eq!(metrics.total_requests, 0);
    }

    #[test]
    fn test_set_env() {
        let mut runner = BotRunner::new();
        runner.set_env("API_KEY", "test123");

        assert_eq!(
            runner.config.env_vars.get("API_KEY"),
            Some(&"test123".to_string())
        );
    }

    #[test]
    fn test_set_timeout() {
        let mut runner = BotRunner::new();
        runner.set_timeout(Duration::from_secs(60));

        assert_eq!(runner.config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_session_info() {
        let runner = BotRunner::new();
        let customer = Customer::default();
        let customer_id = customer.id;

        let session_id = runner.start_session(customer).unwrap();
        let info = runner.get_session_info(session_id).unwrap();

        assert_eq!(info.session_id, session_id);
        assert_eq!(info.customer_id, customer_id);
        assert_eq!(info.message_count, 0);
        assert_eq!(info.state, ConversationState::Initial);
    }

    #[test]
    fn test_log_level_default() {
        let level = LogLevel::default();
        assert_eq!(level, LogLevel::Info);
    }
}
