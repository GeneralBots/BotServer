use botsecurity::protection::{ProtectionManager, ProtectionTool};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityToolResult {
    pub tool: String,
    pub success: bool,
    pub installed: bool,
    pub version: Option<String>,
    pub running: Option<bool>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScanResult {
    pub tool: String,
    pub success: bool,
    pub status: String,
    pub findings_count: usize,
    pub warnings_count: usize,
    pub score: Option<i32>,
    pub report_path: Option<String>,
}

pub async fn security_tool_status(
    _state: Arc<AppState>,
    tool_name: &str,
) -> Result<SecurityToolResult, String> {
    let tool = parse_tool_name(tool_name)?;
    let manager = ProtectionManager::new(ProtectionConfig::default());

    match manager.check_tool_status(tool).await {
        Ok(status) => Ok(SecurityToolResult {
            tool: tool_name.to_lowercase(),
            success: true,
            installed: status.installed,
            version: status.version,
            running: status.service_running,
            message: if status.installed {
                "Tool is installed".to_string()
            } else {
                "Tool is not installed".to_string()
            },
        }),
        Err(e) => Ok(SecurityToolResult {
            tool: tool_name.to_lowercase(),
            success: false,
            installed: false,
            version: None,
            running: None,
            message: format!("Failed to check status: {e}"),
        }),
    }
}

pub async fn security_run_scan(
    _state: Arc<AppState>,
    tool_name: &str,
) -> Result<SecurityScanResult, String> {
    let tool = parse_tool_name(tool_name)?;
    let manager = ProtectionManager::new(ProtectionConfig::default());

    match manager.run_scan(tool).await {
        Ok(result) => Ok(SecurityScanResult {
            tool: tool_name.to_lowercase(),
            success: true,
            status: format!("{:?}", result.status),
            findings_count: result.findings.len(),
            warnings_count: result.warnings as usize,
            score: None,
            report_path: result.report_path,
        }),
        Err(error) => {
            log::error!("Security scan failed for {tool_name}: {error}");
            Ok(SecurityScanResult {
                tool: tool_name.to_lowercase(),
                success: false,
                status: format!("error: {error}"),
                findings_count: 0,
                warnings_count: 0,
                score: None,
                report_path: None,
            })
        }
    }
}

pub async fn security_get_report(
    _state: Arc<AppState>,
    tool_name: &str,
) -> Result<String, String> {
    let tool = parse_tool_name(tool_name)?;
    let manager = ProtectionManager::new(ProtectionConfig::default());

    manager
        .get_report(tool)
        .await
        .map_err(|e| format!("Failed to get report: {e}"))
}

pub async fn security_update_definitions(
    _state: Arc<AppState>,
    tool_name: &str,
) -> Result<SecurityToolResult, String> {
    let tool = parse_tool_name(tool_name)?;
    let manager = ProtectionManager::new(ProtectionConfig::default());

    match manager.update_definitions(tool).await {
        Ok(()) => Ok(SecurityToolResult {
            tool: tool_name.to_lowercase(),
            success: true,
            installed: true,
            version: None,
            running: None,
            message: "Definitions updated successfully".to_string(),
        }),
        Err(e) => Ok(SecurityToolResult {
            tool: tool_name.to_lowercase(),
            success: false,
            installed: true,
            version: None,
            running: None,
            message: format!("Failed to update definitions: {e}"),
        }),
    }
}

pub async fn security_start_service(
    _state: Arc<AppState>,
    tool_name: &str,
) -> Result<SecurityToolResult, String> {
    let tool = parse_tool_name(tool_name)?;
    let manager = ProtectionManager::new(ProtectionConfig::default());

    match manager.start_service(tool).await {
        Ok(()) => Ok(SecurityToolResult {
            tool: tool_name.to_lowercase(),
            success: true,
            installed: true,
            version: None,
            running: Some(true),
            message: "Service started successfully".to_string(),
        }),
        Err(e) => Ok(SecurityToolResult {
            tool: tool_name.to_lowercase(),
            success: false,
            installed: true,
            version: None,
            running: Some(false),
            message: format!("Failed to start service: {e}"),
        }),
    }
}

pub async fn security_stop_service(
    _state: Arc<AppState>,
    tool_name: &str,
) -> Result<SecurityToolResult, String> {
    let tool = parse_tool_name(tool_name)?;
    let manager = ProtectionManager::new(ProtectionConfig::default());

    match manager.stop_service(tool).await {
        Ok(()) => Ok(SecurityToolResult {
            tool: tool_name.to_lowercase(),
            success: true,
            installed: true,
            version: None,
            running: Some(false),
            message: "Service stopped successfully".to_string(),
        }),
        Err(e) => Ok(SecurityToolResult {
            tool: tool_name.to_lowercase(),
            success: false,
            installed: true,
            version: None,
            running: None,
            message: format!("Failed to stop service: {e}"),
        }),
    }
}

pub async fn security_install_tool(
    _state: Arc<AppState>,
    tool_name: &str,
) -> Result<SecurityToolResult, String> {
    let tool = parse_tool_name(tool_name)?;
    let manager = ProtectionManager::new(ProtectionConfig::default());

    match manager.install_tool(tool).await {
        Ok(()) => Ok(SecurityToolResult {
            tool: tool_name.to_lowercase(),
            success: true,
            installed: true,
            version: None,
            running: None,
            message: "Tool installed successfully".to_string(),
        }),
        Err(e) => Ok(SecurityToolResult {
            tool: tool_name.to_lowercase(),
            success: false,
            installed: false,
            version: None,
            running: None,
            message: format!("Failed to install tool: {e}"),
        }),
    }
}

pub async fn security_hardening_score(_state: Arc<AppState>) -> Result<i32, String> {
    let manager = ProtectionManager::new(ProtectionConfig::default());

    match manager.run_scan(ProtectionTool::Lynis).await {
        Ok(_result) => Ok(0),
        Err(e) => Err(format!("Failed to get hardening score: {e}")),
    }
}

pub fn security_tool_is_installed(status: &SecurityToolResult) -> bool {
    status.installed
}

pub fn security_service_is_running(status: &SecurityToolResult) -> bool {
    status.running.unwrap_or(false)
}

fn parse_tool_name(name: &str) -> Result<ProtectionTool, String> {
    ProtectionTool::from_str(name)
        .map_err(|_| format!("Unknown security tool: {name}. Valid tools: lynis, rkhunter, chkrootkit, suricata, lmd, clamav"))
}

use botsecurity::protection::manager::ProtectionConfig;
use botcore::shared::state::AppState;
use std::str::FromStr;
use std::sync::Arc;

// Stub register function — security_protection provides async helper functions
// that require Arc<AppState>. Rhai keyword registration is deferred to botserver layer.
pub fn register_security_keywords(
    _state: Arc<dyn botbasic_types::BasicRuntime>,
    _user: botbasic_types::UserSession,
    _engine: &mut rhai::Engine,
) {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_name_valid() {
        assert!(parse_tool_name("lynis").is_ok());
        assert!(parse_tool_name("LYNIS").is_ok());
        assert!(parse_tool_name("Lynis").is_ok());
        assert!(parse_tool_name("rkhunter").is_ok());
        assert!(parse_tool_name("chkrootkit").is_ok());
        assert!(parse_tool_name("suricata").is_ok());
        assert!(parse_tool_name("lmd").is_ok());
        assert!(parse_tool_name("clamav").is_ok());
    }

    #[test]
    fn test_parse_tool_name_invalid() {
        assert!(parse_tool_name("unknown").is_err());
        assert!(parse_tool_name("").is_err());
        assert!(parse_tool_name("invalid_tool").is_err());
    }

    #[test]
    fn test_security_tool_is_installed() {
        let installed = SecurityToolResult {
            tool: "lynis".to_string(),
            success: true,
            installed: true,
            version: Some("3.0.9".to_string()),
            running: None,
            message: "Tool is installed".to_string(),
        };
        assert!(security_tool_is_installed(&installed));

        let not_installed = SecurityToolResult {
            tool: "lynis".to_string(),
            success: true,
            installed: false,
            version: None,
            running: None,
            message: "Tool is not installed".to_string(),
        };
        assert!(!security_tool_is_installed(&not_installed));
    }

    #[test]
    fn test_security_service_is_running() {
        let running = SecurityToolResult {
            tool: "suricata".to_string(),
            success: true,
            installed: true,
            version: None,
            running: Some(true),
            message: "Service running".to_string(),
        };
        assert!(security_service_is_running(&running));

        let stopped = SecurityToolResult {
            tool: "suricata".to_string(),
            success: true,
            installed: true,
            version: None,
            running: Some(false),
            message: "Service stopped".to_string(),
        };
        assert!(!security_service_is_running(&stopped));

        let unknown = SecurityToolResult {
            tool: "lynis".to_string(),
            success: true,
            installed: true,
            version: None,
            running: None,
            message: "No service".to_string(),
        };
        assert!(!security_service_is_running(&unknown));
    }

    #[test]
    fn test_security_scan_result_serialization() {
        let result = SecurityScanResult {
            tool: "lynis".to_string(),
            success: true,
            status: "completed".to_string(),
            findings_count: 5,
            warnings_count: 12,
            score: Some(78),
            report_path: Some("/var/log/lynis-report.dat".to_string()),
        };

        let json = serde_json::to_string(&result).expect("Failed to serialize");
        assert!(json.contains("\"tool\":\"lynis\""));
        assert!(json.contains("\"score\":78"));
    }
}