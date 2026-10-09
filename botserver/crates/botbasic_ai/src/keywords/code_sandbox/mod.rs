pub(crate) use botsecurity::command_guard::SafeCommand;
pub(crate) use botbasic_types::UserSession;
pub(crate) use botbasic_types::BasicRuntime;
pub(crate) use log::{trace, warn};
pub(crate) use rhai::{Dynamic, Engine};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[derive(Default)]
pub enum SandboxRuntime {
    LXC,

    Docker,

    Firecracker,

    #[default]
    Process,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CodeLanguage {
    Python,
    JavaScript,
    Bash,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub enabled: bool,

    pub runtime: SandboxRuntime,

    pub timeout_seconds: u64,

    pub memory_limit_mb: u64,

    pub cpu_limit_percent: u32,

    pub network_enabled: bool,

    pub work_dir: String,

    pub env_vars: HashMap<String, String>,

    pub allowed_paths: Vec<String>,

    pub python_packages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub stdout: String,

    pub stderr: String,

    pub exit_code: i32,

    pub execution_time_ms: u64,

    pub timed_out: bool,

    pub killed: bool,

    pub error: Option<String>,
}

pub struct CodeSandbox {
    config: SandboxConfig,
    session_id: Uuid,
}

use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;
mod part8;

pub use part5::*;
pub use part6::*;
pub use part7::*;
use part8::*;
