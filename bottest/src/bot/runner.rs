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

struct SessionState {
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

impl BotRunner {
    pub fn new() -> Self {
        Self::with_config(BotRunnerConfig::default())
    }

    pub fn with_config(config: BotRunnerConfig) -> Self {
        Self {
            config,
            bot: None,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            script_cache: Arc::new(Mutex::new(HashMap::new())),
            metrics: Arc::new(Mutex::new(RunnerMetrics::default())),
        }
    }

    pub fn with_context(_ctx: &TestContext, config: BotRunnerConfig) -> Self {
        Self::with_config(config)
    }

    pub fn set_bot(&mut self, bot: Bot) -> &mut Self {
        self.bot = Some(bot);
        self
    }

    pub fn load_script(&self, name: &str, content: &str) -> &Self {
        self.script_cache
            .lock()
            .unwrap()
            .insert(name.to_string(), content.to_string());
        self
    }

    pub fn load_script_file(&self, name: &str, path: &PathBuf) -> Result<&Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read script file: {}", path.display()))?;
        self.script_cache
            .lock()
            .unwrap()
            .insert(name.to_string(), content);
        Ok(self)
    }

    pub fn start_session(&self, customer: Customer) -> Result<Uuid> {
        let session_id = Uuid::new_v4();
        let bot_id = self.bot.as_ref().map_or_else(Uuid::new_v4, |b| b.id);

        let session = Session {
            id: session_id,
            bot_id,
            customer_id: customer.id,
            channel: customer.channel,
            ..Default::default()
        };

        let state = SessionState {
            session,
            channel: customer.channel,
            customer,
            context: HashMap::new(),
            conversation_state: ConversationState::Initial,
            message_count: 0,
            started_at: Instant::now(),
        };

        self.sessions.lock().unwrap().insert(session_id, state);

        Ok(session_id)
    }

    pub fn end_session(&self, session_id: Uuid) -> Result<()> {
        self.sessions.lock().unwrap().remove(&session_id);
        Ok(())
    }

    pub async fn process_message(
        &self,
        session_id: Uuid,
        message: &str,
    ) -> Result<ExecutionResult> {
        let start = Instant::now();
        let mut logs = Vec::new();

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.total_requests += 1;
        }

        let state = {
            let sessions = self.sessions.lock().unwrap();
            sessions.get(&session_id).cloned()
        };

        let Some(state) = state else {
            return Ok(ExecutionResult {
                session_id,
                response: None,
                state: ConversationState::Error,
                execution_time: start.elapsed(),
                logs,
                error: Some("Session not found".to_string()),
            });
        };

        if self.config.capture_logs {
            logs.push(LogEntry {
                timestamp: chrono::Utc::now(),
                level: LogLevel::Debug,
                message: format!("Processing message: {message}"),
                context: HashMap::new(),
            });
        }

        let response = self.execute_bot_logic(session_id, message, &state).await;

        let execution_time = start.elapsed();

        {
            let mut metrics = self.metrics.lock().unwrap();
            let latency_ms = execution_time.as_millis() as u64;
            metrics.total_latency_ms += latency_ms;

            if metrics.min_latency_ms == 0 || latency_ms < metrics.min_latency_ms {
                metrics.min_latency_ms = latency_ms;
            }
            if latency_ms > metrics.max_latency_ms {
                metrics.max_latency_ms = latency_ms;
            }

            if response.is_ok() {
                metrics.successful_requests += 1;
            } else {
                metrics.failed_requests += 1;
            }
        }

        {
            let mut sessions = self.sessions.lock().unwrap();
            if let Some(session_state) = sessions.get_mut(&session_id) {
                session_state.message_count += 1;
                session_state.conversation_state = ConversationState::WaitingForUser;
            }
        }

        match response {
            Ok(bot_response) => Ok(ExecutionResult {
                session_id,
                response: Some(bot_response),
                state: ConversationState::WaitingForUser,
                execution_time,
                logs,
                error: None,
            }),
            Err(e) => Ok(ExecutionResult {
                session_id,
                response: None,
                state: ConversationState::Error,
                execution_time,
                logs,
                error: Some(e.to_string()),
            }),
        }
    }

    async fn execute_bot_logic(
        &self,
        session_id: Uuid,
        message: &str,
        state: &SessionState,
    ) -> Result<BotResponse> {
        let start = Instant::now();

        let bot = self.bot.as_ref().context("No bot configured")?;

        let script_path = self
            .config
            .working_dir
            .join(&bot.name)
            .join("dialog")
            .join("start.bas");

        let script_content = if script_path.exists() {
            tokio::fs::read_to_string(&script_path)
                .await
                .unwrap_or_default()
        } else {
            let cache = self.script_cache.lock().unwrap();
            cache.get("default").cloned().unwrap_or_default()
        };

        let response_content = if script_content.is_empty() {
            format!("Received: {message}")
        } else {
            Self::evaluate_basic_script(&script_content, message, &state.context)
                .unwrap_or_else(|e| format!("Error: {e}"))
        };

        let latency = start.elapsed().as_millis() as u64;

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.total_requests += 1;
            metrics.successful_requests += 1;
            metrics.total_latency_ms += latency;
        }

        Ok(BotResponse {
            id: Uuid::new_v4(),
            content: response_content,
            content_type: ResponseContentType::Text,
            metadata: HashMap::from([
                (
                    "session_id".to_string(),
                    serde_json::Value::String(session_id.to_string()),
                ),
                (
                    "bot_name".to_string(),
                    serde_json::Value::String(bot.name.clone()),
                ),
            ]),
            latency_ms: latency,
        })
    }

    fn evaluate_basic_script(
        script: &str,
        input: &str,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let mut output = String::new();
        let mut variables: HashMap<String, String> = HashMap::new();

        variables.insert("INPUT".to_string(), input.to_string());
        for (key, value) in context {
            variables.insert(key.to_uppercase(), value.to_string());
        }

        for line in script.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('\'') || line.starts_with("REM") {
                continue;
            }

            if line.to_uppercase().starts_with("TALK") {
                let content = line[4..].trim().trim_matches('"');
                let expanded = Self::expand_variables(content, &variables);
                if !output.is_empty() {
                    output.push('\n');
                }
                output.push_str(&expanded);
            } else if line.to_uppercase().starts_with("HEAR") {
                variables.insert("LAST_INPUT".to_string(), input.to_string());
            } else if line.contains('=') && !line.to_uppercase().starts_with("IF") {
                let parts: Vec<&str> = line.splitn(2, '=').collect();
                if parts.len() == 2 {
                    let var_name = parts[0].trim().to_uppercase();
                    let var_value = parts[1].trim().trim_matches('"').to_string();
                    let expanded = Self::expand_variables(&var_value, &variables);
                    variables.insert(var_name, expanded);
                }
            }
        }

        if output.is_empty() {
            output = format!("Processed: {input}");
        }

        Ok(output)
    }

    fn expand_variables(text: &str, variables: &HashMap<String, String>) -> String {
        let mut result = text.to_string();
        for (key, value) in variables {
            result = result.replace(&format!("{{{key}}}"), value);
            result = result.replace(&format!("${key}"), value);
            result = result.replace(key, value);
        }
        result
    }

    pub fn execute_script(&self, script_name: &str, input: &str) -> Result<ExecutionResult> {
        let session_id = Uuid::new_v4();
        let start = Instant::now();
        let mut logs = Vec::new();

        let script = {
            let cache = self.script_cache.lock().unwrap();
            cache.get(script_name).cloned()
        };

        let Some(script) = script else {
            return Ok(ExecutionResult {
                session_id,
                response: None,
                state: ConversationState::Error,
                execution_time: start.elapsed(),
                logs,
                error: Some(format!("Script '{script_name}' not found")),
            });
        };

        if self.config.capture_logs {
            logs.push(LogEntry {
                timestamp: chrono::Utc::now(),
                level: LogLevel::Debug,
                message: format!("Executing script: {script_name}"),
                context: HashMap::new(),
            });
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.script_executions += 1;
        }

        let result = Self::execute_script_internal(&script, input);

        let execution_time = start.elapsed();

        match result {
            Ok(output) => Ok(ExecutionResult {
                session_id,
                response: Some(BotResponse {
                    id: Uuid::new_v4(),
                    content: output,
                    content_type: ResponseContentType::Text,
                    metadata: HashMap::new(),
                    latency_ms: execution_time.as_millis() as u64,
                }),
                state: ConversationState::WaitingForUser,
                execution_time,
                logs,
                error: None,
            }),
            Err(e) => Ok(ExecutionResult {
                session_id,
                response: None,
                state: ConversationState::Error,
                execution_time,
                logs,
                error: Some(e.to_string()),
            }),
        }
    }

    fn execute_script_internal(script: &str, input: &str) -> Result<String> {
        let context = HashMap::new();
        Self::evaluate_basic_script(script, input, &context)
    }

    pub fn metrics(&self) -> RunnerMetrics {
        self.metrics.lock().unwrap().clone()
    }

    pub fn reset_metrics(&self) {
        *self.metrics.lock().unwrap() = RunnerMetrics::default();
    }

    pub fn active_session_count(&self) -> usize {
        self.sessions.lock().unwrap().len()
    }

    pub fn get_session_info(&self, session_id: Uuid) -> Option<SessionInfo> {
        let sessions = self.sessions.lock().unwrap();
        let info = sessions.get(&session_id).map(|s| SessionInfo {
            session_id: s.session.id,
            customer_id: s.customer.id,
            channel: s.channel,
            message_count: s.message_count,
            state: s.conversation_state,
            duration: s.started_at.elapsed(),
        });
        drop(sessions);
        info
    }

    pub fn set_env(&mut self, key: &str, value: &str) -> &mut Self {
        self.config
            .env_vars
            .insert(key.to_string(), value.to_string());
        self
    }

    pub const fn set_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.config.timeout = timeout;
        self
    }
}

impl Default for BotRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: Uuid,
    pub customer_id: Uuid,
    pub channel: Channel,
    pub message_count: usize,
    pub state: ConversationState,
    pub duration: Duration,
}

impl Clone for SessionState {
    fn clone(&self) -> Self {
        Self {
            session: self.session.clone(),
            customer: self.customer.clone(),
            channel: self.channel,
            context: self.context.clone(),
            conversation_state: self.conversation_state,
            message_count: self.message_count,
            started_at: self.started_at,
        }
    }
}

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
