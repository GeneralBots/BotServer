use super::*;

impl ProtectionManager {
    pub async fn update_definitions(&self, tool: ProtectionTool) -> Result<()> {
        info!("Updating definitions for {tool}");

        match tool {
            ProtectionTool::RKHunter => {
                SafeCommand::new("sudo")?
                    .arg("rkhunter")?
                    .arg("--update")?
                    .execute()?;
            }
            ProtectionTool::ClamAV => {
                SafeCommand::new("sudo")?
                    .arg("freshclam")?
                    .execute()?;
            }
            ProtectionTool::Suricata => {
                SafeCommand::new("sudo")?
                    .arg("suricata-update")?
                    .execute()?;
            }
            ProtectionTool::LMD => {
                SafeCommand::new("sudo")?
                    .arg("maldet")?
                    .arg("--update-sigs")?
                    .execute()?;
            }
            _ => {
                return Err(anyhow::anyhow!("{tool} does not support definition updates"));
            }
        }

        if let Some(status) = self.tool_status.write().await.get_mut(&tool) {
            status.last_update = Some(Utc::now());
        }

        Ok(())
    }

    pub async fn set_auto_update(&self, tool: ProtectionTool, enabled: bool) -> Result<()> {
        if let Some(status) = self.tool_status.write().await.get_mut(&tool) {
            status.auto_update = enabled;
        }
        Ok(())
    }

    pub async fn set_auto_remediate(&self, tool: ProtectionTool, enabled: bool) -> Result<()> {
        if let Some(status) = self.tool_status.write().await.get_mut(&tool) {
            status.auto_remediate = enabled;
        }
        Ok(())
    }

    pub async fn get_scan_history(&self, tool: Option<ProtectionTool>, limit: usize) -> Vec<ScanResult> {
        let history = self.scan_history.read().await;
        history
            .iter()
            .filter(|s| tool.is_none() || Some(s.tool) == tool)
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    pub async fn get_active_scans(&self) -> Vec<ScanResult> {
        self.active_scans.read().await.values().cloned().collect()
    }

    pub async fn get_report(&self, tool: ProtectionTool) -> Result<String> {
        let history = self.scan_history.read().await;
        let latest = history
            .iter().rfind(|s| s.tool == tool)
            .ok_or_else(|| anyhow::anyhow!("No scan results found for {tool}"))?;

        latest.raw_output.clone().ok_or_else(|| anyhow::anyhow!("No report available"))
    }
}
