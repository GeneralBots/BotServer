use anyhow::{Context, Result};
use tracing::{info, warn};

use botsecurity_core::command_guard::SafeCommand;
use super::manager::{Finding, FindingSeverity, ScanResultStatus};

const LYNIS_REPORT_PATH: &str = "/var/log/lynis-report.dat";

pub async fn run_scan() -> Result<(ScanResultStatus, Vec<Finding>, String)> {
    info!("Running Lynis security audit");

    let output = SafeCommand::new("sudo")?
        .arg("lynis")?
        .arg("audit")?
        .arg("system")?
        .arg("--quick")?
        .arg("--no-colors")?
        .execute()
        .context("Failed to run Lynis audit")?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let raw_output = format!("{stdout}\n{stderr}");

    let findings = parse_lynis_output(&stdout);
    let status = determine_result_status(&findings);

    Ok((status, findings, raw_output))
}

pub async fn run_full_audit() -> Result<(ScanResultStatus, Vec<Finding>, String)> {
    info!("Running full Lynis security audit");

    let output = SafeCommand::new("sudo")?
        .arg("lynis")?
        .arg("audit")?
        .arg("system")?
        .arg("--no-colors")?
        .execute()
        .context("Failed to run full Lynis audit")?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let raw_output = format!("{stdout}\n{stderr}");

    let findings = parse_lynis_output(&stdout);
    let status = determine_result_status(&findings);

    Ok((status, findings, raw_output))
}

pub fn parse_lynis_output(output: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut current_category = String::new();

    for line in output.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.contains(']') {
            if let Some(category) = extract_category(trimmed) {
                current_category = category;
            }
        }

        if trimmed.contains("Warning:") || trimmed.contains("WARNING") {
            let finding = Finding {
                id: format!("lynis-warn-{}", findings.len()),
                severity: FindingSeverity::Medium,
                category: current_category.clone(),
                title: "Security Warning".to_string(),
                description: trimmed.replace("Warning:", "").trim().to_string(),
                file_path: None,
                remediation: None,
            };
            findings.push(finding);
        }

        if trimmed.contains("Suggestion:") || trimmed.contains("SUGGESTION") {
            let finding = Finding {
                id: format!("lynis-sugg-{}", findings.len()),
                severity: FindingSeverity::Low,
                category: current_category.clone(),
                title: "Security Suggestion".to_string(),
                description: trimmed.replace("Suggestion:", "").trim().to_string(),
                file_path: None,
                remediation: extract_remediation(trimmed),
            };
            findings.push(finding);
        }

        if trimmed.contains("[FOUND]") && trimmed.contains("vulnerable") {
            let finding = Finding {
                id: format!("lynis-vuln-{}", findings.len()),
                severity: FindingSeverity::High,
                category: current_category.clone(),
                title: "Vulnerability Found".to_string(),
                description: trimmed.to_string(),
                file_path: None,
                remediation: None,
            };
            findings.push(finding);
        }
    }

    findings
}

pub fn parse_report_file() -> Result<LynisReport> {
    let content = std::fs::read_to_string(LYNIS_REPORT_PATH)
        .context("Failed to read Lynis report file")?;

    let mut report = LynisReport::default();

    for line in content.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            match key {
                "hardening_index" => {
                    report.hardening_index = value.parse().unwrap_or(0);
                }
                "warning[]" => {
                    report.warnings.push(value.to_string());
                }
                "suggestion[]" => {
                    report.suggestions.push(value.to_string());
                }
                "lynis_version" => {
                    report.version = value.to_string();
                }
                "test_category[]" => {
                    report.categories_tested.push(value.to_string());
                }
                "tests_executed" => {
                    report.tests_executed = value.parse().unwrap_or(0);
                }
                _ => {}
            }
        }
    }

    Ok(report)
}

pub async fn get_hardening_index() -> Result<u32> {
    let report = parse_report_file()?;
    Ok(report.hardening_index)
}

pub async fn apply_suggestion(suggestion_id: &str) -> Result<()> {
    info!("Applying Lynis suggestion: {suggestion_id}");

    warn!("Auto-remediation for suggestion {suggestion_id} not yet implemented");

    Ok(())
}

/// Extracts the section name of a Lynis header such as `[+] Boot and services`.
///
/// Lynis writes the pass/fail marker inside the brackets and the section name
/// after them. Stopping at the first `]` would reduce every section to a bare
/// "+" and leave findings uncategorised, so the marker and the name are joined
/// back together without the surrounding brackets.
fn extract_category(line: &str) -> Option<String> {
    let start = line.find('[')?;
    let end = line.find(']')?;
    if start >= end {
        return None;
    }

    let marker = &line[start + 1..end];
    let category = format!("{marker}{}", &line[end + 1..]);
    let category = category.trim();
    if category.is_empty() {
        None
    } else {
        Some(category.to_string())
    }
}

fn extract_remediation(line: &str) -> Option<String> {
    if line.contains("Consider") {
        Some(line.split("Consider").nth(1)?.trim().to_string())
    } else if line.contains("Disable") {
        Some(line.split("Disable").nth(1).map(|s| format!("Disable {}", s.trim()))?)
    } else if line.contains("Enable") {
        Some(line.split("Enable").nth(1).map(|s| format!("Enable {}", s.trim()))?)
    } else {
        None
    }
}

fn determine_result_status(findings: &[Finding]) -> ScanResultStatus {
    let has_critical = findings.iter().any(|f| f.severity == FindingSeverity::Critical);
    let has_high = findings.iter().any(|f| f.severity == FindingSeverity::High);
    let has_medium = findings.iter().any(|f| f.severity == FindingSeverity::Medium);

    if has_critical || has_high {
        ScanResultStatus::Infected
    } else if has_medium {
        ScanResultStatus::Warnings
    } else {
        ScanResultStatus::Clean
    }
}

#[derive(Debug, Clone, Default)]
pub struct LynisReport {
    pub version: String,
    pub hardening_index: u32,
    pub tests_executed: u32,
    pub warnings: Vec<String>,
    pub suggestions: Vec<String>,
    pub categories_tested: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lynis_output_warnings() {
        let output = r#"
[+] Boot and services
    - Service Manager                                        [ systemd ]
    Warning: Some warning message here
[+] Kernel
    Suggestion: Consider enabling some feature
"#;
        let findings = parse_lynis_output(output);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].severity, FindingSeverity::Medium);
        assert_eq!(findings[1].severity, FindingSeverity::Low);
    }

    #[test]
    fn test_extract_category() {
        assert_eq!(extract_category("[+] Boot and services"), Some("+ Boot and services".to_string()));
        assert_eq!(extract_category("no brackets"), None);
    }

    #[test]
    fn test_determine_result_status_clean() {
        let findings: Vec<Finding> = vec![];
        assert_eq!(determine_result_status(&findings), ScanResultStatus::Clean);
    }

    #[test]
    fn test_determine_result_status_warnings() {
        let findings = vec![Finding {
            id: "test".to_string(),
            severity: FindingSeverity::Medium,
            category: "test".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            file_path: None,
            remediation: None,
        }];
        assert_eq!(determine_result_status(&findings), ScanResultStatus::Warnings);
    }

    #[test]
    fn test_determine_result_status_infected() {
        let findings = vec![Finding {
            id: "test".to_string(),
            severity: FindingSeverity::High,
            category: "test".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            file_path: None,
            remediation: None,
        }];
        assert_eq!(determine_result_status(&findings), ScanResultStatus::Infected);
    }

    #[test]
    fn test_lynis_report_default() {
        let report = LynisReport::default();
        assert_eq!(report.hardening_index, 0);
        assert!(report.warnings.is_empty());
        assert!(report.suggestions.is_empty());
    }
}
