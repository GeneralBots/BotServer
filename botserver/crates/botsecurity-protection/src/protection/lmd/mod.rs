pub(crate) use anyhow::{Context, Result};
pub(crate) use tracing::info;
pub(crate) use botsecurity_core::command_guard::SafeCommand;
pub(crate) use super::manager::{Finding, FindingSeverity, ScanResultStatus};

const LMD_QUARANTINE_DIR: &str = "/usr/local/maldetect/quarantine";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QuarantinedFile {
    pub id: String,
    pub original_path: String,
    pub quarantine_path: String,
    pub size: u64,
    pub quarantined_at: Option<chrono::DateTime<chrono::Utc>>,
    pub threat_name: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct LMDStats {
    pub signature_count: u64,
    pub quarantined_count: u32,
    pub last_scan: Option<chrono::DateTime<chrono::Utc>>,
    pub threats_found: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lmd_output_clean() {
        let output = r#"
Linux Malware Detect v1.6.5
Scanning /var/www
Total files scanned: 1234
Total hits: 0
Total cleaned: 0
"#;
        let findings = parse_lmd_output(output);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_parse_lmd_output_hit() {
        let output = r#"
Linux Malware Detect v1.6.5
Scanning /var/www
{HIT} /var/www/uploads/shell.php : php.cmdshell.unclassed.6
Total hits: 1
"#;
        let findings = parse_lmd_output(output);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, FindingSeverity::Critical);
    }

    #[test]
    fn test_parse_lmd_output_suspicious() {
        let output = r#"
Linux Malware Detect v1.6.5
Scanning /var/www
suspicious file found: /var/www/uploads/unknown.php
"#;
        let findings = parse_lmd_output(output);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, FindingSeverity::High);
    }

    #[test]
    fn test_extract_scan_id() {
        assert_eq!(
            extract_scan_id("scan id: 123456.789"),
            Some("123456.789".to_string())
        );
        assert_eq!(
            extract_scan_id("SCAN ID: abc123"),
            Some("abc123".to_string())
        );
        assert_eq!(extract_scan_id("no scan id here"), None);
    }

    #[test]
    fn test_extract_file_path_from_line() {
        assert_eq!(
            extract_file_path_from_line("Found malware in /var/www/shell.php"),
            Some("/var/www/shell.php".to_string())
        );
        assert_eq!(
            extract_file_path_from_line("No path here"),
            None
        );
    }

    #[test]
    fn test_extract_original_path() {
        assert_eq!(
            extract_original_path("var.www.uploads.shell.php"),
            "var/www/uploads/shell/php"
        );
    }

    #[test]
    fn test_determine_result_status_clean() {
        let findings: Vec<Finding> = vec![];
        assert_eq!(determine_result_status(&findings), ScanResultStatus::Clean);
    }

    #[test]
    fn test_determine_result_status_infected() {
        let findings = vec![Finding {
            id: "test".to_string(),
            severity: FindingSeverity::Critical,
            category: "test".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            file_path: None,
            remediation: None,
        }];
        assert_eq!(determine_result_status(&findings), ScanResultStatus::Infected);
    }

    #[test]
    fn test_determine_result_status_warnings() {
        let findings = vec![Finding {
            id: "test".to_string(),
            severity: FindingSeverity::High,
            category: "test".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            file_path: None,
            remediation: None,
        }];
        assert_eq!(determine_result_status(&findings), ScanResultStatus::Warnings);
    }

    #[test]
    fn test_quarantined_file_struct() {
        let file = QuarantinedFile {
            id: "test".to_string(),
            original_path: "/var/www/shell.php".to_string(),
            quarantine_path: "/usr/local/maldetect/quarantine/test".to_string(),
            size: 1024,
            quarantined_at: None,
            threat_name: Some("php.cmdshell".to_string()),
        };
        assert_eq!(file.size, 1024);
        assert!(file.threat_name.is_some());
    }

    #[test]
    fn test_lmd_stats_default() {
        let stats = LMDStats::default();
        assert_eq!(stats.signature_count, 0);
        assert_eq!(stats.quarantined_count, 0);
        assert!(stats.last_scan.is_none());
    }

    #[test]
    fn test_parse_lmd_output_warning() {
        let output = r#"
Linux Malware Detect v1.6.5
Warning: signature database may be outdated
Scanning /var/www
"#;
        let findings = parse_lmd_output(output);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, FindingSeverity::Medium);
    }

    #[test]
    fn test_parse_lmd_output_found() {
        let output = r#"
FOUND: /var/www/malicious.php : malware.backdoor.123
"#;
        let findings = parse_lmd_output(output);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, FindingSeverity::Critical);
    }
}

mod body;

pub use body::*;
