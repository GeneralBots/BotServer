pub(crate) use botsecurity::command_guard::SafeCommand;
pub(crate) use botbasic_types::BasicRuntime;
pub(crate) use chrono::{DateTime, Utc};
pub(crate) use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub id: String,

    pub name: String,

    pub description: String,

    pub server_type: McpServerType,

    pub connection: McpConnection,

    pub auth: McpAuth,

    pub tools: Vec<McpTool>,

    pub capabilities: McpCapabilities,

    pub status: McpServerStatus,

    pub bot_id: String,

    pub created_at: DateTime<Utc>,

    pub updated_at: DateTime<Utc>,

    pub last_health_check: Option<DateTime<Utc>>,

    pub health_status: HealthStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum McpServerType {
    Database,
    Filesystem,
    Web,
    Email,
    Slack,
    Teams,
    Analytics,
    Search,
    Storage,
    Compute,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConnection {
    pub connection_type: ConnectionType,

    pub url: String,

    pub port: Option<u16>,

    pub timeout_seconds: i32,

    pub max_retries: i32,

    pub retry_backoff_ms: i32,

    pub keep_alive: bool,

    pub tls_config: Option<TlsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[derive(Default)]
pub enum ConnectionType {
    #[default]
    Http,

    WebSocket,

    Grpc,

    UnixSocket,

    Stdio,

    Tcp,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub enabled: bool,
    pub verify_certificates: bool,
    pub ca_cert_path: Option<String>,
    pub client_cert_path: Option<String>,
    pub client_key_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpAuth {
    pub auth_type: McpAuthType,

    pub credentials: McpCredentials,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[derive(Default)]
pub enum McpAuthType {
    #[default]
    None,
    ApiKey,
    Bearer,
    Basic,
    OAuth2,
    Certificate,
    Custom(String),
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub enum McpCredentials {
    #[default]
    None,
    ApiKey {
        header_name: String,
        key_ref: String,
    },
    Bearer {
        token_ref: String,
    },
    Basic {
        username_ref: String,
        password_ref: String,
    },
    OAuth2 {
        client_id_ref: String,
        client_secret_ref: String,
        token_url: String,
        scopes: Vec<String>,
    },
    Certificate {
        cert_ref: String,
        key_ref: String,
    },
    Custom(HashMap<String, String>),
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,

    pub description: String,

    pub input_schema: serde_json::Value,

    pub output_schema: Option<serde_json::Value>,

    pub required_permissions: Vec<String>,

    pub risk_level: ToolRiskLevel,

    pub is_destructive: bool,

    pub requires_approval: bool,

    pub rate_limit: Option<i32>,

    pub timeout_seconds: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[derive(Default)]
pub enum ToolRiskLevel {
    Safe,
    #[default]
    Low,
    Medium,
    High,
    Critical,
}


#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpCapabilities {
    pub tools: bool,

    pub resources: bool,

    pub prompts: bool,

    pub logging: bool,

    pub streaming: bool,

    pub cancellation: bool,

    pub custom: HashMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[derive(Default)]
pub enum McpServerStatus {
    Active,
    #[default]
    Inactive,
    Connecting,
    Error(String),
    Maintenance,
    Unknown,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct HealthStatus {
    pub healthy: bool,
    pub last_check: Option<DateTime<Utc>>,
    pub response_time_ms: Option<i64>,
    pub error_message: Option<String>,
    pub consecutive_failures: i32,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub id: String,

    pub server: String,

    pub tool: String,

    pub arguments: serde_json::Value,

    pub context: McpRequestContext,

    pub timeout_seconds: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequestContext {
    pub session_id: String,
    pub bot_id: String,
    pub user_id: String,
    pub task_id: Option<String>,
    pub step_id: Option<String>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    pub id: String,

    pub success: bool,

    pub result: Option<serde_json::Value>,

    pub error: Option<McpError>,

    pub metadata: McpResponseMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub retryable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponseMetadata {
    pub duration_ms: i64,
    pub server_version: Option<String>,
    pub rate_limit_remaining: Option<i32>,
    pub rate_limit_reset: Option<DateTime<Utc>>,
}

pub struct McpClient {
    state: Arc<dyn BasicRuntime>,
    config: McpClientConfig,
    servers: HashMap<String, McpServer>,
    http_client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpClientConfig {
    pub enabled: bool,

    pub default_timeout_seconds: i32,

    pub max_concurrent_requests: i32,

    pub cache_enabled: bool,

    pub cache_ttl_seconds: i32,

    pub audit_enabled: bool,

    pub health_check_interval_seconds: i32,

    pub auto_retry: bool,

    pub circuit_breaker_threshold: i32,
}

use diesel::prelude::*;
use log::info;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
