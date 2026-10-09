use super::*;

impl ProtectionManager {
    pub fn new(config: ProtectionConfig) -> Self {
        Self {
            config,
            tool_status: Arc::new(RwLock::new(HashMap::new())),
            active_scans: Arc::new(RwLock::new(HashMap::new())),
            scan_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing Protection Manager");
        for tool in &self.config.enabled_tools {
            let status = self.check_tool_status(*tool).await?;
            self.tool_status.write().await.insert(*tool, status);
        }
        Ok(())
    }

    pub async fn check_tool_status(&self, tool: ProtectionTool) -> Result<ToolStatus> {
        let installed = self.is_tool_installed(tool).await;

        if !installed {
            return Ok(ToolStatus::not_installed(tool));
        }

        let version = self.get_tool_version(tool).await.ok();
        let service_running = if tool.service_name().is_some() {
            Some(self.is_service_running(tool).await)
        } else {
            None
        };

        let stored = self.tool_status.read().await;
        let existing = stored.get(&tool);

        Ok(ToolStatus {
            tool,
            installed: true,
            version,
            service_running,
            last_scan: existing.and_then(|s| s.last_scan),
            last_update: existing.and_then(|s| s.last_update),
            auto_update: existing.map(|s| s.auto_update).unwrap_or(false),
            auto_remediate: existing.map(|s| s.auto_remediate).unwrap_or(false),
            metrics: existing.map(|s| s.metrics.clone()).unwrap_or_default(),
        })
    }

    pub async fn is_tool_installed(&self, tool: ProtectionTool) -> bool {
        let binary = tool.binary_name();

        let result = SafeCommand::new("which")
            .and_then(|cmd| cmd.arg(binary))
            .and_then(|cmd| cmd.execute());

        match result {
            Ok(output) => output.status.success(),
            Err(e) => {
                warn!("Failed to check if {tool} is installed: {e}");
                false
            }
        }
    }

    pub async fn get_tool_version(&self, tool: ProtectionTool) -> Result<String> {
        let binary = tool.binary_name();
        let version_arg = match tool {
            ProtectionTool::Lynis => "--version",
            ProtectionTool::RKHunter => "--version",
            ProtectionTool::Chkrootkit => "-V",
            ProtectionTool::Suricata => "--build-info",
            ProtectionTool::LMD => "--version",
            ProtectionTool::ClamAV => "--version",
        };

        let output = SafeCommand::new(binary)?
            .arg(version_arg)?
            .execute()
            .context("Failed to get tool version")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let version = stdout.lines().next().unwrap_or("unknown").trim().to_string();
        Ok(version)
    }

    pub async fn is_service_running(&self, tool: ProtectionTool) -> bool {
        let Some(service_name) = tool.service_name() else {
            return false;
        };

        let result = SafeCommand::new("sudo")
            .and_then(|cmd| cmd.arg("systemctl"))
            .and_then(|cmd| cmd.arg("is-active"))
            .and_then(|cmd| cmd.arg(service_name));

        match result {
            Ok(cmd) => match cmd.execute() {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    stdout.trim() == "active"
                }
                Err(_) => false,
            },
            Err(_) => false,
        }
    }

    pub async fn get_all_status(&self) -> HashMap<ProtectionTool, ToolStatus> {
        self.tool_status.read().await.clone()
    }

    pub async fn get_tool_status_by_name(&self, name: &str) -> Option<ToolStatus> {
        let tool: ProtectionTool = name.parse().ok()?;
        self.tool_status.read().await.get(&tool).cloned()
    }

    pub async fn install_tool(&self, tool: ProtectionTool) -> Result<()> {
        info!("Installing protection tool: {tool}");

        let package = tool.package_name();

        SafeCommand::new("apt-get")?
            .arg("install")?
            .arg("-y")?
            .arg(package)?
            .execute()
            .context("Failed to install tool")?;

        let status = self.check_tool_status(tool).await?;
        self.tool_status.write().await.insert(tool, status);

        info!("Successfully installed {tool}");
        Ok(())
    }

    pub async fn start_service(&self, tool: ProtectionTool) -> Result<()> {
        let Some(service_name) = tool.service_name() else {
            return Err(anyhow::anyhow!("{tool} does not have a service"));
        };

        info!("Starting service: {service_name}");

        SafeCommand::new("sudo")?
            .arg("systemctl")?
            .arg("start")?
            .arg(service_name)?
            .execute()
            .context("Failed to start service")?;

        if let Some(status) = self.tool_status.write().await.get_mut(&tool) {
            status.service_running = Some(true);
        }

        Ok(())
    }

    pub async fn stop_service(&self, tool: ProtectionTool) -> Result<()> {
        let Some(service_name) = tool.service_name() else {
            return Err(anyhow::anyhow!("{tool} does not have a service"));
        };

        info!("Stopping service: {service_name}");

        SafeCommand::new("sudo")?
            .arg("systemctl")?
            .arg("stop")?
            .arg(service_name)?
            .execute()
            .context("Failed to stop service")?;

        if let Some(status) = self.tool_status.write().await.get_mut(&tool) {
            status.service_running = Some(false);
        }

        Ok(())
    }

    pub async fn enable_service(&self, tool: ProtectionTool) -> Result<()> {
        let Some(service_name) = tool.service_name() else {
            return Err(anyhow::anyhow!("{tool} does not have a service"));
        };

        SafeCommand::new("sudo")?
            .arg("systemctl")?
            .arg("enable")?
            .arg(service_name)?
            .execute()
            .context("Failed to enable service")?;

        Ok(())
    }

    pub async fn disable_service(&self, tool: ProtectionTool) -> Result<()> {
        let Some(service_name) = tool.service_name() else {
            return Err(anyhow::anyhow!("{tool} does not have a service"));
        };

        SafeCommand::new("sudo")?
            .arg("systemctl")?
            .arg("disable")?
            .arg(service_name)?
            .execute()
            .context("Failed to disable service")?;

        Ok(())
    }

    pub async fn run_scan(&self, tool: ProtectionTool) -> Result<ScanResult> {
        let scan_id = uuid::Uuid::new_v4().to_string();
        let started_at = Utc::now();

        info!("Starting {tool} scan with ID: {scan_id}");

        let mut result = ScanResult {
            scan_id: scan_id.clone(),
            tool,
            started_at,
            completed_at: None,
            status: ScanStatus::Running,
            result: ScanResultStatus::Unknown,
            findings: Vec::new(),
            warnings: 0,
            report_path: None,
            raw_output: None,
        };

        self.active_scans.write().await.insert(scan_id.clone(), result.clone());

        let scan_output = match tool {
            ProtectionTool::Lynis => super::lynis::run_scan().await,
            ProtectionTool::RKHunter => super::rkhunter::run_scan().await,
            ProtectionTool::Chkrootkit => super::chkrootkit::run_scan().await,
            ProtectionTool::Suricata => super::suricata::get_alerts().await,
            ProtectionTool::LMD => super::lmd::run_scan(None).await,
            ProtectionTool::ClamAV => super::lynis::run_scan().await,
        };

        result.completed_at = Some(Utc::now());

        match scan_output {
            Ok((status, findings, raw)) => {
                result.status = ScanStatus::Completed;
                result.result = status;
                result.warnings = findings.iter().filter(|f| f.severity == FindingSeverity::Medium || f.severity == FindingSeverity::Low).count() as u32;
                result.findings = findings;
                result.raw_output = Some(raw);
            }
            Err(e) => {
                warn!("Scan failed for {tool}: {e}");
                result.status = ScanStatus::Failed;
                result.raw_output = Some(e.to_string());
            }
        }

        self.active_scans.write().await.remove(&scan_id);

        if let Some(status) = self.tool_status.write().await.get_mut(&tool) {
            status.last_scan = Some(Utc::now());
            status.metrics.warnings = result.warnings;
            status.metrics.threats_found = result.findings.iter()
                .filter(|f| f.severity == FindingSeverity::High || f.severity == FindingSeverity::Critical)
                .count() as u32;
        }

        self.scan_history.write().await.push(result.clone());

        Ok(result)
    }
}
