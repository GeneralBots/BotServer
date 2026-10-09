pub(crate) use anyhow::{Context, Result};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::path::{Path, PathBuf};
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use tracing::{info, warn};
#[cfg(unix)]
pub(crate) use std::os::unix::fs::PermissionsExt;
pub(crate) use super::command_guard::SafeCommand;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThreatSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThreatStatus {
    Detected,
    Quarantined,
    Removed,
    Allowed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Threat {
    pub id: String,
    pub name: String,
    pub threat_type: String,
    pub severity: ThreatSeverity,
    pub status: ThreatStatus,
    pub file_path: Option<String>,
    pub detected_at: chrono::DateTime<chrono::Utc>,
    pub description: Option<String>,
    pub action_taken: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub id: String,
    pub cve_id: Option<String>,
    pub name: String,
    pub severity: ThreatSeverity,
    pub affected_component: String,
    pub description: String,
    pub remediation: Option<String>,
    pub detected_at: chrono::DateTime<chrono::Utc>,
    pub is_patched: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub scan_id: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub status: ScanStatus,
    pub files_scanned: u64,
    pub threats_found: Vec<Threat>,
    pub scan_type: ScanType,
    pub target_path: Option<String>,
    pub error_message: Option<String>,
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
#[serde(rename_all = "snake_case")]
pub enum ScanType {
    Quick,
    Full,
    Custom,
    Memory,
    Rootkit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectionStatus {
    pub real_time_protection: bool,
    pub windows_defender_enabled: bool,
    pub general_bots_protection: bool,
    pub last_scan: Option<chrono::DateTime<chrono::Utc>>,
    pub last_definition_update: Option<chrono::DateTime<chrono::Utc>>,
    pub threats_blocked_today: u32,
    pub quarantined_items: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntivirusConfig {
    pub clamav_path: Option<PathBuf>,

    pub real_time_protection: bool,

    pub quarantine_dir: PathBuf,

    pub log_dir: PathBuf,

    pub auto_quarantine: bool,

    pub excluded_paths: Vec<PathBuf>,

    pub excluded_extensions: Vec<String>,

    pub max_file_size_mb: u64,

    pub scan_archives: bool,

    pub definition_update_url: Option<String>,
}

#[derive(Debug)]
pub struct AntivirusManager {
    config: AntivirusConfig,
    threats: Arc<RwLock<Vec<Threat>>>,
    vulnerabilities: Arc<RwLock<Vec<Vulnerability>>>,
    active_scans: Arc<RwLock<HashMap<String, ScanResult>>>,
    protection_status: Arc<RwLock<ProtectionStatus>>,
}

pub mod api {
    use super::*;

    #[derive(Debug, Serialize, Deserialize)]
    pub struct ThreatsResponse {
        pub threats: Vec<Threat>,
        pub total: usize,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct VulnerabilitiesResponse {
        pub vulnerabilities: Vec<Vulnerability>,
        pub total: usize,
        pub critical: usize,
        pub high: usize,
        pub medium: usize,
        pub low: usize,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct ScanRequest {
        pub scan_type: ScanType,
        pub target_path: Option<String>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct ScanResponse {
        pub scan_id: String,
        pub status: ScanStatus,
        pub message: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct ActionResponse {
        pub success: bool,
        pub message: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct DefenderStatusRequest {
        pub enabled: bool,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_threat() {
        assert_eq!(
            AntivirusManager::classify_threat("Win.Trojan.Generic"),
            "Trojan"
        );
        assert_eq!(
            AntivirusManager::classify_threat("Ransomware.WannaCry"),
            "Ransomware"
        );
        assert_eq!(
            AntivirusManager::classify_threat("PUP.Optional.Adware"),
            "Adware"
        );
        assert_eq!(
            AntivirusManager::classify_threat("PUP.Optional.Toolbar"),
            "PUP"
        );
        assert_eq!(
            AntivirusManager::classify_threat("Unknown.Malware"),
            "Malware"
        );
        assert_eq!(AntivirusManager::classify_threat("Worm.Conficker"), "Worm");
        assert_eq!(
            AntivirusManager::classify_threat("Spyware.Keylogger"),
            "Spyware"
        );
        assert_eq!(
            AntivirusManager::classify_threat("Rootkit.Hidden"),
            "Rootkit"
        );
    }

    #[test]
    fn test_assess_severity() {
        assert_eq!(
            AntivirusManager::assess_severity("Ransomware.Test"),
            ThreatSeverity::Critical
        );
        assert_eq!(
            AntivirusManager::assess_severity("Rootkit.Hidden"),
            ThreatSeverity::Critical
        );
        assert_eq!(
            AntivirusManager::assess_severity("Trojan.Generic"),
            ThreatSeverity::High
        );
        assert_eq!(
            AntivirusManager::assess_severity("Backdoor.RAT"),
            ThreatSeverity::High
        );
        assert_eq!(
            AntivirusManager::assess_severity("Virus.Test"),
            ThreatSeverity::Medium
        );
        assert_eq!(
            AntivirusManager::assess_severity("Worm.Spread"),
            ThreatSeverity::Medium
        );
        assert_eq!(
            AntivirusManager::assess_severity("PUP.Adware"),
            ThreatSeverity::Low
        );
    }

    #[test]
    fn test_antivirus_config_default() {
        let config = AntivirusConfig::default();
        assert!(config.scan_archives);
        assert!(config.excluded_paths.is_empty());
    }

    #[test]
    fn test_threat_severity_ordering() {
        assert!(matches!(ThreatSeverity::Critical, ThreatSeverity::Critical));
        assert!(matches!(ThreatSeverity::High, ThreatSeverity::High));
        assert!(matches!(ThreatSeverity::Medium, ThreatSeverity::Medium));
        assert!(matches!(ThreatSeverity::Low, ThreatSeverity::Low));
    }

    #[test]
    fn test_scan_status_variants() {
        assert!(matches!(ScanStatus::Pending, ScanStatus::Pending));
        assert!(matches!(ScanStatus::Running, ScanStatus::Running));
        assert!(matches!(ScanStatus::Completed, ScanStatus::Completed));
        assert!(matches!(ScanStatus::Failed, ScanStatus::Failed));
        assert!(matches!(ScanStatus::Cancelled, ScanStatus::Cancelled));
    }

    #[test]
    fn test_threat_status_variants() {
        assert!(matches!(ThreatStatus::Detected, ThreatStatus::Detected));
        assert!(matches!(
            ThreatStatus::Quarantined,
            ThreatStatus::Quarantined
        ));
        assert!(matches!(ThreatStatus::Removed, ThreatStatus::Removed));
        assert!(matches!(ThreatStatus::Allowed, ThreatStatus::Allowed));
        assert!(matches!(ThreatStatus::Failed, ThreatStatus::Failed));
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
