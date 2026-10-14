//! Tests for basic module
//!
//! Extracted from mod.rs to reduce file size

#[cfg(test)]
use std::collections::HashMap;
use std::time::Duration;

// Test script constants from bottest/fixtures/scripts/mod.rs

const GREETING_SCRIPT: &str = r#"
' Greeting Flow Script
' Simple greeting and response pattern

REM Initialize greeting
greeting$ = "Hello! Welcome to our service."
TALK greeting$

REM Wait for user response
HEAR userInput$

REM Check for specific keywords
IF INSTR(UCASE$(userInput$), "HELP") > 0 THEN
    TALK "I can help you with: Products, Support, or Billing. What would you like to know?"
ELSEIF INSTR(UCASE$(userInput$), "BYE") > 0 THEN
    TALK "Goodbye! Have a great day!"
    END
ELSE
    TALK "Thank you for your message. How can I assist you today?"
END IF
"#;

const SIMPLE_ECHO_SCRIPT: &str = r#"
' Simple Echo Script
' Echoes back whatever user says

TALK "Echo Bot: I will repeat everything you say. Type 'quit' to exit."

echo_loop:
HEAR input$

IF UCASE$(input$) = "QUIT" THEN
    TALK "Goodbye!"
    END
END IF

TALK "You said: " + input$
GOTO echo_loop
"#;

const VARIABLES_SCRIPT: &str = r#"
' Variables and Expressions Script
' Demonstrates variable types and operations

REM String variables
firstName$ = "John"
lastName$ = "Doe"
fullName$ = firstName$ + " " + lastName$
TALK "Full name: " + fullName$

REM Numeric variables
price = 99.99
quantity = 3
subtotal = price * quantity
tax = subtotal * 0.08
total = subtotal + tax
TALK "Total: $" + STR$(total)
"#;

fn get_script(name: &str) -> Option<&'static str> {
    match name {
        "greeting" => Some(GREETING_SCRIPT),
        "simple_echo" => Some(SIMPLE_ECHO_SCRIPT),
        "variables" => Some(VARIABLES_SCRIPT),
        _ => None,
    }
}

fn available_scripts() -> Vec<&'static str> {
    vec!["greeting", "simple_echo", "variables"]
}

fn all_scripts() -> HashMap<&'static str, &'static str> {
    let mut scripts = HashMap::new();
    for name in available_scripts() {
        if let Some(content) = get_script(name) {
            scripts.insert(name, content);
        }
    }
    scripts
}

// Runner types from bottest/bot/runner.rs

#[derive(Debug, Clone)]
pub struct BotRunnerConfig {
    pub working_dir: std::path::PathBuf,
    pub timeout: Duration,
    pub use_mocks: bool,
    pub env_vars: HashMap<String, String>,
    pub capture_logs: bool,
    pub log_level: LogLevel,
}

impl BotRunnerConfig {
    pub const fn log_level(&self) -> LogLevel {
        self.log_level
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
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

    pub const fn min_latency(&self) -> u64 {
        self.min_latency_ms
    }

    pub const fn max_latency(&self) -> u64 {
        self.max_latency_ms
    }

    pub const fn latency_range(&self) -> u64 {
        self.max_latency_ms.saturating_sub(self.min_latency_ms)
    }
}

// Tests

#[test]
fn test_get_script() {
    assert!(get_script("greeting").is_some());
    assert!(get_script("simple_echo").is_some());
    assert!(get_script("nonexistent").is_none());
}

#[test]
fn test_available_scripts() {
    let scripts = available_scripts();
    assert!(!scripts.is_empty());
    assert!(scripts.contains(&"greeting"));
}

#[test]
fn test_all_scripts() {
    let scripts = all_scripts();
    assert_eq!(scripts.len(), available_scripts().len());
}

#[test]
fn test_greeting_script_content() {
    let script = get_script("greeting").unwrap();
    assert!(script.contains("TALK"));
    assert!(script.contains("HEAR"));
    assert!(script.contains("greeting"));
}

#[test]
fn test_simple_echo_script_content() {
    let script = get_script("simple_echo").unwrap();
    assert!(script.contains("HEAR"));
    assert!(script.contains("TALK"));
    assert!(script.contains("GOTO"));
}

#[test]
fn test_variables_script_content() {
    let script = get_script("variables").unwrap();
    assert!(script.contains("firstName$"));
    assert!(script.contains("price"));
    assert!(script.contains("STR$"));
}

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
        ..RunnerMetrics::default()
    };

    assert_eq!(metrics.avg_latency_ms(), 100);
}

#[test]
fn test_runner_metrics_success_rate() {
    let metrics = RunnerMetrics {
        total_requests: 100,
        successful_requests: 95,
        ..RunnerMetrics::default()
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
fn test_log_level_default() {
    let level = LogLevel::default();
    assert_eq!(level, LogLevel::Info);
}

#[test]
fn test_runner_config_env_vars() {
    let mut env_vars = HashMap::new();
    env_vars.insert("API_KEY".to_string(), "test123".to_string());
    env_vars.insert("DEBUG".to_string(), "true".to_string());

    let config = BotRunnerConfig {
        env_vars,
        ..BotRunnerConfig::default()
    };

    assert_eq!(config.env_vars.get("API_KEY"), Some(&"test123".to_string()));
    assert_eq!(config.env_vars.get("DEBUG"), Some(&"true".to_string()));
}

#[test]
fn test_runner_config_timeout() {
    let config = BotRunnerConfig {
        timeout: Duration::from_secs(60),
        ..BotRunnerConfig::default()
    };

    assert_eq!(config.timeout, Duration::from_secs(60));
}

#[test]
fn test_metrics_tracking() {
    let metrics = RunnerMetrics {
        total_requests: 50,
        successful_requests: 45,
        failed_requests: 5,
        total_latency_ms: 5000,
        min_latency_ms: 10,
        max_latency_ms: 500,
        ..RunnerMetrics::default()
    };

    assert_eq!(metrics.avg_latency_ms(), 100);
    assert!((metrics.success_rate() - 90.0).abs() < f64::EPSILON);
    assert_eq!(
        metrics.total_requests,
        metrics.successful_requests + metrics.failed_requests
    );
    assert_eq!(metrics.min_latency(), 10);
    assert_eq!(metrics.max_latency(), 500);
    assert_eq!(metrics.latency_range(), 490);
}

#[test]
fn test_script_execution_tracking() {
    let metrics = RunnerMetrics {
        script_executions: 25,
        transfer_to_human_count: 3,
        ..RunnerMetrics::default()
    };

    assert_eq!(metrics.script_executions, 25);
    assert_eq!(metrics.transfer_to_human_count, 3);
}

#[test]
fn test_log_level_accessor() {
    let config = BotRunnerConfig::default();
    assert_eq!(config.log_level(), LogLevel::Info);
}

#[test]
fn test_log_levels() {
    assert!(matches!(LogLevel::Trace, LogLevel::Trace));
    assert!(matches!(LogLevel::Debug, LogLevel::Debug));
    assert!(matches!(LogLevel::Info, LogLevel::Info));
    assert!(matches!(LogLevel::Warn, LogLevel::Warn));
    assert!(matches!(LogLevel::Error, LogLevel::Error));
}

#[test]
fn test_script_contains_basic_keywords() {
    for name in available_scripts() {
        if let Some(script) = get_script(name) {
            // All scripts should have some form of output
            let has_output = script.contains("TALK") || script.contains("PRINT");
            assert!(has_output, "Script {} should have output keyword", name);
        }
    }
}

#[test]
fn test_runner_config_working_dir() {
    let config = BotRunnerConfig::default();
    assert!(config.working_dir.to_str().unwrap_or_default().contains("bottest"));
}
