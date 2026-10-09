use super::*;

impl AntivirusManager {
    pub fn new(config: AntivirusConfig) -> Result<Self> {
        std::fs::create_dir_all(&config.quarantine_dir)
            .context("Failed to create quarantine directory")?;
        std::fs::create_dir_all(&config.log_dir).context("Failed to create log directory")?;

        let protection_status = ProtectionStatus {
            real_time_protection: config.real_time_protection,
            windows_defender_enabled: Self::check_windows_defender_status(),
            general_bots_protection: true,
            last_scan: None,
            last_definition_update: None,
            threats_blocked_today: 0,
            quarantined_items: 0,
        };

        Ok(Self {
            config,
            threats: Arc::new(RwLock::new(Vec::new())),
            vulnerabilities: Arc::new(RwLock::new(Vec::new())),
            active_scans: Arc::new(RwLock::new(HashMap::new())),
            protection_status: Arc::new(RwLock::new(protection_status)),
        })
    }

    #[cfg(target_os = "windows")]
    pub(super) fn check_windows_defender_status() -> bool {
        let result = SafeCommand::new("powershell")
            .and_then(|cmd| cmd.arg("-Command"))
            .and_then(|cmd| cmd.arg("Get-MpPreference | Select-Object -ExpandProperty DisableRealtimeMonitoring"))
            .and_then(|cmd| cmd.execute());

        match result {
            Ok(output) => {
                let result = String::from_utf8_lossy(&output.stdout);
                !result.trim().eq_ignore_ascii_case("true")
            }
            Err(e) => {
                warn!("Failed to check Windows Defender status: {}", e);
                false
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub(super) fn check_windows_defender_status() -> bool {
        false
    }

    #[cfg(target_os = "windows")]
    pub async fn disable_windows_defender(&self) -> Result<bool> {
        info!("Attempting to disable Windows Defender...");

        let script = r#"
            Set-MpPreference -DisableRealtimeMonitoring $true
            Set-MpPreference -DisableBehaviorMonitoring $true
            Set-MpPreference -DisableBlockAtFirstSeen $true
            Set-MpPreference -DisableIOAVProtection $true
            Set-MpPreference -DisablePrivacyMode $true
            Set-MpPreference -SignatureDisableUpdateOnStartupWithoutEngine $true
            Set-MpPreference -DisableArchiveScanning $true
            Set-MpPreference -DisableIntrusionPreventionSystem $true
            Set-MpPreference -DisableScriptScanning $true
        "#;

        let output = SafeCommand::new("powershell")
            .and_then(|c| c.arg("-Command"))
            .and_then(|c| c.arg(script))
            .map_err(|e| anyhow::anyhow!("Failed to build PowerShell command: {}", e))?
            .execute()
            .context("Failed to execute PowerShell command")?;

        if output.status.success() {
            let mut status = self.protection_status.write().await;
            status.windows_defender_enabled = false;
            info!("Windows Defender disabled successfully");
            Ok(true)
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            log::error!("Failed to disable Windows Defender: {}", error);
            Err(anyhow::anyhow!(
                "Failed to disable Windows Defender: {}",
                error
            ))
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn disable_windows_defender(&self) -> Result<bool> {
        warn!("Windows Defender management is only available on Windows");
        Ok(false)
    }

    #[cfg(target_os = "windows")]
    pub async fn enable_windows_defender(&self) -> Result<bool> {
        info!("Attempting to enable Windows Defender...");

        let script = r#"
            Set-MpPreference -DisableRealtimeMonitoring $false
            Set-MpPreference -DisableBehaviorMonitoring $false
            Set-MpPreference -DisableBlockAtFirstSeen $false
            Set-MpPreference -DisableIOAVProtection $false
            Set-MpPreference -DisableArchiveScanning $false
            Set-MpPreference -DisableIntrusionPreventionSystem $false
            Set-MpPreference -DisableScriptScanning $false
        "#;

        let output = SafeCommand::new("powershell")
            .and_then(|c| c.arg("-Command"))
            .and_then(|c| c.arg(script))
            .map_err(|e| anyhow::anyhow!("Failed to build PowerShell command: {}", e))?
            .execute()
            .context("Failed to execute PowerShell command")?;

        if output.status.success() {
            let mut status = self.protection_status.write().await;
            status.windows_defender_enabled = true;
            info!("Windows Defender enabled successfully");
            Ok(true)
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            log::error!("Failed to enable Windows Defender: {}", error);
            Err(anyhow::anyhow!(
                "Failed to enable Windows Defender: {}",
                error
            ))
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn enable_windows_defender(&self) -> Result<bool> {
        warn!("Windows Defender management is only available on Windows");
        Ok(false)
    }

    pub async fn start_scan(
        &self,
        scan_type: ScanType,
        target_path: Option<&str>,
    ) -> Result<String> {
        let scan_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let scan_result = ScanResult {
            scan_id: scan_id.clone(),
            started_at: now,
            completed_at: None,
            status: ScanStatus::Pending,
            files_scanned: 0,
            threats_found: vec![],
            scan_type,
            target_path: target_path.map(|s| s.to_string()),
            error_message: None,
        };

        {
            let mut scans = self.active_scans.write().await;
            scans.insert(scan_id.clone(), scan_result);
        }

        let scan_id_clone = scan_id.clone();
        let scans = self.active_scans.clone();
        let threats = self.threats.clone();
        let config = self.config.clone();
        let target = target_path.map(|s| s.to_string());

        tokio::spawn(async move {
            Self::run_scan(scan_id_clone, scan_type, target, scans, threats, config).await;
        });

        info!("Started {:?} scan with ID: {}", scan_type, scan_id);
        Ok(scan_id)
    }

    async fn run_scan(
        scan_id: String,
        scan_type: ScanType,
        target_path: Option<String>,
        scans: Arc<RwLock<HashMap<String, ScanResult>>>,
        threats: Arc<RwLock<Vec<Threat>>>,
        config: AntivirusConfig,
    ) {
        {
            let mut scans_guard = scans.write().await;
            if let Some(scan) = scans_guard.get_mut(&scan_id) {
                scan.status = ScanStatus::Running;
            }
        }

        let scan_path = match scan_type {
            ScanType::Quick => target_path.unwrap_or_else(|| {
                if cfg!(target_os = "windows") {
                    "C:\\Users".to_string()
                } else {
                    "/home".to_string()
                }
            }),
            ScanType::Full => target_path.unwrap_or_else(|| {
                if cfg!(target_os = "windows") {
                    "C:\\".to_string()
                } else {
                    "/".to_string()
                }
            }),
            ScanType::Custom => target_path.unwrap_or_else(|| ".".to_string()),
            ScanType::Memory => "memory".to_string(),
            ScanType::Rootkit => "/".to_string(),
        };

        let result = Self::run_clamav_scan(&scan_path, &config);

        let mut scans_guard = scans.write().await;
        if let Some(scan) = scans_guard.get_mut(&scan_id) {
            scan.completed_at = Some(chrono::Utc::now());

            match result {
                Ok((files_scanned, found_threats)) => {
                    scan.status = ScanStatus::Completed;
                    scan.files_scanned = files_scanned;
                    scan.threats_found.clone_from(&found_threats);

                    if !found_threats.is_empty() {
                        let mut threats_guard = threats.blocking_write();
                        threats_guard.extend(found_threats);
                    }
                }
                Err(e) => {
                    scan.status = ScanStatus::Failed;
                    scan.error_message = Some(e.to_string());
                }
            }
        }
    }
}
