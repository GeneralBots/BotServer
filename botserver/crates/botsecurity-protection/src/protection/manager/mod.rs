pub(crate) use anyhow::{Context, Result};
pub(crate) use chrono::{DateTime, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use tracing::{info, warn};
pub(crate) use botsecurity_core::command_guard::SafeCommand;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProtectionTool {
    Lynis,
    RKHunter,
    Chkrootkit,
    Suricata,
    LMD,
    ClamAV,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatus {
    pub tool: ProtectionTool,
    pub installed: bool,
    pub version: Option<String>,
    pub service_running: Option<bool>,
    pub last_scan: Option<DateTime<Utc>>,
    pub last_update: Option<DateTime<Utc>>,
    pub auto_update: bool,
    pub auto_remediate: bool,
    pub metrics: ToolMetrics,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolMetrics {
    pub hardening_index: Option<u32>,
    pub warnings: u32,
    pub suggestions: u32,
    pub threats_found: u32,
    pub rules_count: Option<u32>,
    pub alerts_today: u32,
    pub blocked_today: u32,
    pub signatures_count: Option<u64>,
    pub quarantined_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub scan_id: String,
    pub tool: ProtectionTool,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: ScanStatus,
    pub result: ScanResultStatus,
    pub findings: Vec<Finding>,
    pub warnings: u32,
    pub report_path: Option<String>,
    pub raw_output: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanResultStatus {
    Clean,
    Warnings,
    Infected,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub severity: FindingSeverity,
    pub category: String,
    pub title: String,
    pub description: String,
    pub file_path: Option<String>,
    pub remediation: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FindingSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectionConfig {
    pub enabled_tools: Vec<ProtectionTool>,
    pub auto_scan_interval_hours: u32,
    pub auto_update_interval_hours: u32,
    pub quarantine_dir: String,
    pub log_dir: String,
}

pub struct ProtectionManager {
    config: ProtectionConfig,
    tool_status: Arc<RwLock<HashMap<ProtectionTool, ToolStatus>>>,
    active_scans: Arc<RwLock<HashMap<String, ScanResult>>>,
    scan_history: Arc<RwLock<Vec<ScanResult>>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protection_tool_from_str() {
        assert_eq!("lynis".parse::<ProtectionTool>(), Ok(ProtectionTool::Lynis));
        assert_eq!("LYNIS".parse::<ProtectionTool>(), Ok(ProtectionTool::Lynis));
        assert_eq!("rkhunter".parse::<ProtectionTool>(), Ok(ProtectionTool::RKHunter));
        assert_eq!("clamav".parse::<ProtectionTool>(), Ok(ProtectionTool::ClamAV));
        assert_eq!("clamscan".parse::<ProtectionTool>(), Ok(ProtectionTool::ClamAV));
        assert_eq!("maldet".parse::<ProtectionTool>(), Ok(ProtectionTool::LMD));
        assert!("unknown".parse::<ProtectionTool>().is_err());
    }

    #[test]
    fn test_protection_tool_display() {
        assert_eq!(format!("{}", ProtectionTool::Lynis), "lynis");
        assert_eq!(format!("{}", ProtectionTool::ClamAV), "clamav");
    }

    #[test]
    fn test_tool_status_not_installed() {
        let status = ToolStatus::not_installed(ProtectionTool::Lynis);
        assert!(!status.installed);
        assert!(status.version.is_none());
        assert!(status.service_running.is_none());
    }

    #[test]
    fn test_protection_config_default() {
        let config = ProtectionConfig::default();
        assert_eq!(config.auto_scan_interval_hours, 24);
        assert_eq!(config.auto_update_interval_hours, 6);
        assert_eq!(config.enabled_tools.len(), 6);
    }

    #[test]
    fn test_protection_tool_all() {
        let all = ProtectionTool::all();
        assert_eq!(all.len(), 6);
        assert!(all.contains(&ProtectionTool::Lynis));
        assert!(all.contains(&ProtectionTool::ClamAV));
    }

    #[test]
    fn test_finding_severity() {
        let finding = Finding {
            id: "test".to_string(),
            severity: FindingSeverity::High,
            category: "security".to_string(),
            title: "Test".to_string(),
            description: "Test finding".to_string(),
            file_path: None,
            remediation: None,
        };
        assert_eq!(finding.severity, FindingSeverity::High);
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
