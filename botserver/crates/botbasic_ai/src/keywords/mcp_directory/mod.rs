pub(crate) use chrono::Utc;
pub(crate) use log::{debug, info};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use super::mcp_client::{ ConnectionType, HealthStatus, McpAuth, McpCapabilities, McpConnection, McpServer, McpServerStatus, McpServerType, };

#[derive(Debug, Clone)]
pub struct McpCsvRow {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,

    #[serde(default)]
    pub description: String,

    #[serde(rename = "type")]
    pub server_type: String,

    #[serde(default = "default_enabled")]
    pub enabled: bool,

    pub connection: McpConnectionConfig,

    #[serde(default)]
    pub auth: Option<McpAuthConfig>,

    #[serde(default)]
    pub tools: Vec<McpToolConfig>,

    #[serde(default)]
    pub env: HashMap<String, String>,

    #[serde(default)]
    pub tags: Vec<String>,

    #[serde(default)]
    pub risk_level: String,

    #[serde(default)]
    pub requires_approval: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpConnectionConfig {
    #[serde(rename = "stdio")]
    Stdio {
        command: String,

        #[serde(default)]
        args: Vec<String>,

        #[serde(default)]
        cwd: Option<String>,
    },

    #[serde(rename = "http")]
    Http {
        url: String,

        #[serde(default = "default_timeout")]
        timeout: u32,

        #[serde(default)]
        headers: HashMap<String, String>,
    },

    #[serde(rename = "websocket")]
    WebSocket {
        url: String,

        #[serde(default = "default_timeout")]
        timeout: u32,
    },

    #[serde(rename = "tcp")]
    Tcp { host: String, port: u16 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpAuthConfig {
    #[serde(rename = "none")]
    None,

    #[serde(rename = "api_key")]
    ApiKey {
        #[serde(default = "default_api_key_header")]
        header: String,

        key_env: String,
    },

    #[serde(rename = "bearer")]
    Bearer { token_env: String },

    #[serde(rename = "basic")]
    Basic {
        username_env: String,

        password_env: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolConfig {
    pub name: String,

    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub input_schema: Option<serde_json::Value>,

    #[serde(default)]
    pub output_schema: Option<serde_json::Value>,

    #[serde(default)]
    pub risk_level: Option<String>,

    #[serde(default)]
    pub requires_approval: bool,

    #[serde(default)]
    pub rate_limit: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct McpLoadResult {
    pub servers: Vec<McpServer>,

    pub errors: Vec<McpLoadError>,

    pub lines_processed: usize,

    pub file_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct McpLoadError {
    pub line: usize,

    pub message: String,

    pub recoverable: bool,
}

#[derive(Debug)]
pub struct McpCsvLoader {
    work_path: String,

    bot_id: String,
}

pub type McpDirectoryScanner = McpCsvLoader;
pub type McpDirectoryScanResult = McpLoadResult;
pub type McpScanError = McpLoadError;

use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;

use part1::*;
use part2::*;
use part3::*;
pub use part6::*;
