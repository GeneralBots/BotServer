pub(crate) use axum::{ extract::Path, http::StatusCode, routing::{get, post}, Json, Router, };
pub(crate) use botcore::AppState;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::str::FromStr;
pub(crate) use std::sync::Arc;
pub(crate) use std::sync::OnceLock;
pub(crate) use tokio::sync::RwLock;
pub(crate) use tracing::warn;
pub(crate) use super::manager::{ProtectionConfig, ProtectionManager, ProtectionTool, ScanResult, ToolStatus};
pub(crate) use super::security_fix::{run_security_fix, run_security_status, SecurityFixReport};
// Split fallout (#1492): `api/part*.rs` refers to sibling tool
// modules through `super::`, which now means `api`.
pub(crate) use super::lmd;

static PROTECTION_MANAGER: OnceLock<Arc<RwLock<ProtectionManager>>> = OnceLock::new();

#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct AllStatusResponse {
    tools: Vec<ToolStatus>,
}

#[derive(Debug, Deserialize)]
struct AutoToggleRequest {
    enabled: bool,
    setting: Option<String>,
}

#[derive(Debug, Serialize)]
struct ActionResponse {
    success: bool,
    message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_valid() {
        assert!(parse_tool("lynis").is_ok());
        assert!(parse_tool("rkhunter").is_ok());
        assert!(parse_tool("clamav").is_ok());
        assert!(parse_tool("LYNIS").is_ok());
    }

    #[test]
    fn test_parse_tool_invalid() {
        assert!(parse_tool("unknown").is_err());
        assert!(parse_tool("").is_err());
    }

    #[test]
    fn test_api_response_success() {
        let response = ApiResponse::success("test data");
        assert!(response.success);
        assert!(response.data.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let response: ApiResponse<()> = ApiResponse::error("test error".to_string());
        assert!(!response.success);
        assert!(response.data.is_none());
        assert!(response.error.is_some());
    }

    #[test]
    fn test_action_response() {
        let response = ActionResponse {
            success: true,
            message: "Test message".to_string(),
        };
        assert!(response.success);
        assert_eq!(response.message, "Test message");
    }

    #[test]
    fn test_auto_toggle_request_deserialize() {
        let json = r#"{"enabled": true, "setting": "update"}"#;
        let request: AutoToggleRequest = serde_json::from_str(json).unwrap();
        assert!(request.enabled);
        assert_eq!(request.setting, Some("update".to_string()));
    }

    #[test]
    fn test_auto_toggle_request_minimal() {
        let json = r#"{"enabled": false}"#;
        let request: AutoToggleRequest = serde_json::from_str(json).unwrap();
        assert!(!request.enabled);
        assert!(request.setting.is_none());
    }
}

mod part1;
mod part2;
mod part3;
mod part4;

use part1::*;
pub use part3::*;
use part4::*;
