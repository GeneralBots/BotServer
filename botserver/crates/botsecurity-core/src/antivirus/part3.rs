use super::*;

impl AntivirusManager {
    pub(super) fn run_clamav_scan(path: &str, config: &AntivirusConfig) -> Result<(u64, Vec<Threat>)> {
        let clamscan = config
            .clamav_path
            .clone()
            .map(|p| p.join("clamscan"))
            .unwrap_or_else(|| {
                if cfg!(target_os = "windows") {
                    PathBuf::from("C:\\Program Files\\ClamAV\\clamscan.exe")
                } else {
                    PathBuf::from("/usr/bin/clamscan")
                }
            });

        if !clamscan.exists() {
            let output = SafeCommand::new("which")
                .and_then(|cmd| cmd.arg("clamscan"))
                .and_then(|cmd| cmd.execute())
                .unwrap_or_else(|_| {
                    SafeCommand::new("where")
                        .and_then(|cmd| cmd.arg("clamscan"))
                        .and_then(|cmd| cmd.execute())
                        .unwrap_or_else(|_| std::process::Output {
                            status: std::process::ExitStatus::default(),
                            stdout: vec![],
                            stderr: vec![],
                        })
                });

            if output.stdout.is_empty() {
                return Err(anyhow::anyhow!(
                    "ClamAV not found. Please install ClamAV first."
                ));
            }
        }

        let mut safe_cmd = SafeCommand::new("clamscan")
            .map_err(|e| anyhow::anyhow!("Failed to create safe command: {}", e))?;

        safe_cmd = safe_cmd
            .arg("-r")
            .and_then(|cmd| cmd.arg("--infected"))
            .and_then(|cmd| cmd.arg("--no-summary"))
            .map_err(|e| anyhow::anyhow!("Failed to add arguments: {}", e))?;

        if config.scan_archives {
            safe_cmd = safe_cmd
                .arg("--scan-archive=yes")
                .map_err(|e| anyhow::anyhow!("Failed to add archive arg: {}", e))?;
        }

        for excluded in &config.excluded_paths {
            let exclude_arg = format!("--exclude-dir={}", excluded.display());
            safe_cmd = safe_cmd
                .arg(&exclude_arg)
                .map_err(|e| anyhow::anyhow!("Failed to add exclude arg: {}", e))?;
        }

        safe_cmd = safe_cmd
            .arg(path)
            .map_err(|e| anyhow::anyhow!("Failed to add path arg: {}", e))?;

        let output = safe_cmd
            .execute()
            .map_err(|e| anyhow::anyhow!("Failed to run ClamAV scan: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut threats = Vec::new();
        let mut files_scanned: u64 = 0;

        for line in stdout.lines() {
            if line.contains("FOUND") {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 2 {
                    let file_path = parts[0].trim();
                    let threat_name = parts[1].trim().replace(" FOUND", "");

                    threats.push(Threat {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: threat_name.clone(),
                        threat_type: Self::classify_threat(&threat_name),
                        severity: Self::assess_severity(&threat_name),
                        status: ThreatStatus::Detected,
                        file_path: Some(file_path.to_string()),
                        detected_at: chrono::Utc::now(),
                        description: Some(format!("Detected by ClamAV: {}", threat_name)),
                        action_taken: None,
                    });
                }
            }
            files_scanned += 1;
        }

        Ok((files_scanned, threats))
    }

    pub(super) fn classify_threat(name: &str) -> String {
        let name_lower = name.to_lowercase();
        if name_lower.contains("trojan") {
            "Trojan".to_string()
        } else if name_lower.contains("virus") {
            "Virus".to_string()
        } else if name_lower.contains("worm") {
            "Worm".to_string()
        } else if name_lower.contains("ransomware") {
            "Ransomware".to_string()
        } else if name_lower.contains("spyware") {
            "Spyware".to_string()
        } else if name_lower.contains("adware") {
            "Adware".to_string()
        } else if name_lower.contains("rootkit") {
            "Rootkit".to_string()
        } else if name_lower.contains("pup") || name_lower.contains("pua") {
            "PUP".to_string()
        } else {
            "Malware".to_string()
        }
    }

    pub(super) fn assess_severity(name: &str) -> ThreatSeverity {
        let name_lower = name.to_lowercase();
        if name_lower.contains("ransomware") || name_lower.contains("rootkit") {
            ThreatSeverity::Critical
        } else if name_lower.contains("trojan") || name_lower.contains("backdoor") {
            ThreatSeverity::High
        } else if name_lower.contains("virus") || name_lower.contains("worm") {
            ThreatSeverity::Medium
        } else {
            ThreatSeverity::Low
        }
    }

    pub async fn quarantine_file(&self, file_path: &Path) -> Result<()> {
        if !file_path.exists() {
            return Err(anyhow::anyhow!("File not found: {}", file_path.display()));
        }

        let file_name = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let quarantine_path = self.config.quarantine_dir.join(format!(
            "{}_{}",
            chrono::Utc::now().timestamp(),
            file_name
        ));

        std::fs::rename(file_path, &quarantine_path)
            .context("Failed to move file to quarantine")?;

        info!("File quarantined: {:?} -> {:?}", file_path, quarantine_path);

        let mut status = self.protection_status.write().await;
        status.quarantined_items += 1;

        Ok(())
    }

    pub async fn remove_threat(&self, threat_id: &str) -> Result<()> {
        let mut threats = self.threats.write().await;

        if let Some(pos) = threats.iter().position(|t| t.id == threat_id) {
            let threat = &threats[pos];

            if let Some(ref file_path) = threat.file_path {
                let path = Path::new(file_path);
                if path.exists() {
                    std::fs::remove_file(path).context("Failed to remove infected file")?;
                    info!("Removed infected file: {}", file_path);
                }
            }

            threats[pos].status = ThreatStatus::Removed;
            threats[pos].action_taken = Some("File removed".to_string());
        }

        Ok(())
    }

    pub async fn get_threats(&self) -> Vec<Threat> {
        self.threats.read().await.clone()
    }

    pub async fn get_threats_by_status(&self, status: ThreatStatus) -> Vec<Threat> {
        self.threats
            .read()
            .await
            .iter()
            .filter(|t| t.status == status)
            .cloned()
            .collect()
    }

    pub async fn get_vulnerabilities(&self) -> Vec<Vulnerability> {
        self.vulnerabilities.read().await.clone()
    }

    pub async fn scan_vulnerabilities(&self) -> Result<Vec<Vulnerability>> {
        let mut vulnerabilities = Vec::new();

        #[cfg(target_os = "windows")]
        {
            let script = "Get-HotFix | Sort-Object -Property InstalledOn -Descending | Select-Object -First 1";
            let output = SafeCommand::new("powershell")
                .and_then(|c| c.arg("-Command"))
                .and_then(|c| c.arg(script));

            if let Ok(cmd) = output {
                if let Ok(result) = cmd.execute() {
                    let output_str = String::from_utf8_lossy(&result.stdout);

                    if output_str.is_empty() {
                        vulnerabilities.push(Vulnerability {
                            id: uuid::Uuid::new_v4().to_string(),
                            cve_id: None,
                            name: "Missing Windows Updates".to_string(),
                            severity: ThreatSeverity::High,
                            affected_component: "Windows Update".to_string(),
                            description: "System may be missing critical security updates".to_string(),
                            remediation: Some(
                                "Run Windows Update to install latest patches".to_string(),
                            ),
                            detected_at: chrono::Utc::now(),
                            is_patched: false,
                        });
                    }
                }
            }
        }

        #[cfg(unix)]
        {
            

            let sensitive_paths = vec!["/etc/passwd", "/etc/shadow", "/etc/ssh/sshd_config"];

            for path_str in sensitive_paths {
                let path = Path::new(path_str);
                if path.exists() {
                    if let Ok(metadata) = std::fs::metadata(path) {
                        let mode = metadata.permissions().mode();
                        if mode & 0o002 != 0 {
                            vulnerabilities.push(Vulnerability {
                                id: uuid::Uuid::new_v4().to_string(),
                                cve_id: None,
                                name: format!("Weak permissions on {}", path_str),
                                severity: ThreatSeverity::High,
                                affected_component: path_str.to_string(),
                                description: "Sensitive file has world-writable permissions"
                                    .to_string(),
                                remediation: Some(format!("chmod o-w {}", path_str)),
                                detected_at: chrono::Utc::now(),
                                is_patched: false,
                            });
                        }
                    }
                }
            }
        }

        {
            let mut vulns = self.vulnerabilities.write().await;
            vulns.extend(vulnerabilities.clone());
        }

        Ok(vulnerabilities)
    }

    pub async fn get_protection_status(&self) -> ProtectionStatus {
        self.protection_status.read().await.clone()
    }

    pub async fn get_scan_result(&self, scan_id: &str) -> Option<ScanResult> {
        self.active_scans.read().await.get(scan_id).cloned()
    }

    pub async fn get_all_scans(&self) -> Vec<ScanResult> {
        self.active_scans.read().await.values().cloned().collect()
    }

    pub async fn cancel_scan(&self, scan_id: &str) -> Result<()> {
        let mut scans = self.active_scans.write().await;
        if let Some(scan) = scans.get_mut(scan_id) {
            if scan.status == ScanStatus::Running || scan.status == ScanStatus::Pending {
                scan.status = ScanStatus::Cancelled;
                scan.completed_at = Some(chrono::Utc::now());
                info!("Scan {} cancelled", scan_id);
                Ok(())
            } else {
                Err(anyhow::anyhow!("Scan is not running"))
            }
        } else {
            Err(anyhow::anyhow!("Scan not found"))
        }
    }
}
