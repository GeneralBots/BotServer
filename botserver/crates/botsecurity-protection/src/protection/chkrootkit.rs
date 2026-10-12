use anyhow::{Context, Result};
use tracing::info;

use botsecurity_core::command_guard::SafeCommand;
use super::manager::{Finding, FindingSeverity, ScanResultStatus};

pub async fn run_scan() -> Result<(ScanResultStatus, Vec<Finding>, String)> {
    info!("Running Chkrootkit rootkit scan");

    let output = SafeCommand::new("sudo")?
        .arg("chkrootkit")?
        .arg("-q")?
        .execute()
        .context("Failed to run Chkrootkit scan")?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let raw_output = format!("{stdout}\n{stderr}");

    let findings = parse_chkrootkit_output(&stdout);
    let status = determine_result_status(&findings);

    Ok((status, findings, raw_output))
}

pub async fn run_expert_scan() -> Result<(ScanResultStatus, Vec<Finding>, String)> {
    info!("Running Chkrootkit expert mode scan");

    let output = SafeCommand::new("sudo")?
        .arg("chkrootkit")?
        .arg("-x")?
        .execute()
        .context("Failed to run Chkrootkit expert scan")?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let raw_output = format!("{stdout}\n{stderr}");

    let findings = parse_chkrootkit_output(&stdout);
    let status = determine_result_status(&findings);

    Ok((status, findings, raw_output))
}

pub fn parse_chkrootkit_output(output: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut current_check = String::new();

    for line in output.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with("Checking") || trimmed.starts_with("Searching") {
            current_check = trimmed.to_string();
            // chkrootkit normally appends the verdict to the very same line
            // ("Checking `basename'... INFECTED"), so the line must still be
            // evaluated below instead of being skipped as pure context.
        }

        if trimmed.contains("INFECTED") {
            let finding = Finding {
                id: format!("chkrootkit-infected-{}", findings.len()),
                severity: FindingSeverity::Critical,
                category: "Rootkit Detection".to_string(),
                title: "Infected File or Process Detected".to_string(),
                description: describe(&current_check, trimmed),
                file_path: extract_file_path(trimmed),
                remediation: Some("Immediately investigate and consider system recovery from clean backup".to_string()),
            };
            findings.push(finding);
        }

        if trimmed.contains("Vulnerable") || trimmed.contains("VULNERABLE") {
            let finding = Finding {
                id: format!("chkrootkit-vuln-{}", findings.len()),
                severity: FindingSeverity::High,
                category: "Vulnerability".to_string(),
                title: "Vulnerable Component Detected".to_string(),
                description: describe(&current_check, trimmed),
                file_path: extract_file_path(trimmed),
                remediation: Some("Update the affected component to patch the vulnerability".to_string()),
            };
            findings.push(finding);
        }

        if trimmed.contains("Possible") && trimmed.contains("rootkit") {
            let finding = Finding {
                id: format!("chkrootkit-possible-{}", findings.len()),
                severity: FindingSeverity::High,
                category: "Rootkit Detection".to_string(),
                title: "Possible Rootkit Detected".to_string(),
                description: describe(&current_check, trimmed),
                file_path: extract_file_path(trimmed),
                remediation: Some("Investigate the suspicious activity and verify system integrity".to_string()),
            };
            findings.push(finding);
        }

        if trimmed.contains("Warning") || trimmed.contains("WARNING") {
            let finding = Finding {
                id: format!("chkrootkit-warn-{}", findings.len()),
                severity: FindingSeverity::Medium,
                category: "Security Warning".to_string(),
                title: "Security Warning".to_string(),
                description: trimmed.to_string(),
                file_path: extract_file_path(trimmed),
                remediation: None,
            };
            findings.push(finding);
        }

        if trimmed.contains("suspicious") {
            let finding = Finding {
                id: format!("chkrootkit-susp-{}", findings.len()),
                severity: FindingSeverity::Medium,
                category: current_check.clone(),
                title: "Suspicious Activity Detected".to_string(),
                description: trimmed.to_string(),
                file_path: extract_file_path(trimmed),
                remediation: Some("Review the flagged item for potential threats".to_string()),
            };
            findings.push(finding);
        }
    }

    findings
}

pub async fn get_version() -> Result<String> {
    let output = SafeCommand::new("chkrootkit")?
        .arg("-V")?
        .execute()
        .context("Failed to get Chkrootkit version")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = stdout
        .lines()
        .next()
        .unwrap_or("unknown")
        .trim()
        .to_string();

    Ok(version)
}

/// Builds the finding description, avoiding a duplicated line when chkrootkit
/// reports the verdict on the same line it introduced the check.
fn describe(context: &str, detail: &str) -> String {
    if context == detail {
        detail.to_string()
    } else {
        format!("{context}: {detail}")
    }
}

fn extract_file_path(line: &str) -> Option<String> {
    // chkrootkit quotes the item it is checking: Checking `/usr/bin/ls'...
    // The quotes surround the whole token, so strip them before looking for the
    // leading slash, and again from the tail of the path.
    let words: Vec<&str> = line.split_whitespace().collect();
    for word in words {
        let trimmed = word.trim_matches(|c| c == ':' || c == ',' || c == ';' || c == '`' || c == '\'');
        if trimmed.starts_with('/') {
            return Some(trimmed.to_string());
        }
    }
    None
}

fn determine_result_status(findings: &[Finding]) -> ScanResultStatus {
    let has_critical = findings.iter().any(|f| f.severity == FindingSeverity::Critical);
    let has_high = findings.iter().any(|f| f.severity == FindingSeverity::High);
    let has_medium = findings.iter().any(|f| f.severity == FindingSeverity::Medium);

    if has_critical {
        ScanResultStatus::Infected
    } else if has_high || has_medium {
        ScanResultStatus::Warnings
    } else {
        ScanResultStatus::Clean
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_chkrootkit_output_clean() {
        let output = r#"
Checking `amd'... not found
Checking `basename'... not infected
Checking `biff'... not found
Checking `chfn'... not infected
Checking `chsh'... not infected
Checking `cron'... not infected
"#;
        let findings = parse_chkrootkit_output(output);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_parse_chkrootkit_output_infected() {
        let output = r#"
Checking `amd'... not found
Checking `basename'... INFECTED
Checking `biff'... not found
"#;
        let findings = parse_chkrootkit_output(output);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, FindingSeverity::Critical);
        assert!(findings[0].description.contains("INFECTED"));
    }

    #[test]
    fn test_parse_chkrootkit_output_vulnerable() {
        let output = r#"
Checking `lkm'...
Searching for Suckit rootkit... Vulnerable
"#;
        let findings = parse_chkrootkit_output(output);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, FindingSeverity::High);
    }

    #[test]
    fn test_parse_chkrootkit_output_suspicious() {
        let output = r#"
Checking `sniffer'... lo: not promisc and no packet sniffer sockets
eth0: suspicious activity detected
"#;
        let findings = parse_chkrootkit_output(output);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, FindingSeverity::Medium);
    }

    #[test]
    fn test_extract_file_path() {
        assert_eq!(
            extract_file_path("Found suspicious file: /etc/passwd"),
            Some("/etc/passwd".to_string())
        );
        assert_eq!(
            extract_file_path("Checking `/usr/bin/ls'"),
            Some("/usr/bin/ls".to_string())
        );
        assert_eq!(extract_file_path("No path in this line"), None);
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
    fn test_parse_chkrootkit_possible_rootkit() {
        let output = r#"
Checking `sniffer'... Possible rootkit activity detected
"#;
        let findings = parse_chkrootkit_output(output);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, FindingSeverity::High);
        assert!(findings[0].title.contains("Possible Rootkit"));
    }

    #[test]
    fn test_parse_chkrootkit_warning() {
        let output = r#"
Warning: some security issue detected
"#;
        let findings = parse_chkrootkit_output(output);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, FindingSeverity::Medium);
    }
}
