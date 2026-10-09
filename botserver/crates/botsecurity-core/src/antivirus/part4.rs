use super::*;

impl AntivirusManager {
    pub async fn update_definitions(&self) -> Result<()> {
        info!("Updating virus definitions...");

        let freshclam = if cfg!(target_os = "windows") {
            "freshclam.exe"
        } else {
            "freshclam"
        };

        let output = SafeCommand::new(freshclam)
            .and_then(|cmd| cmd.execute())
            .map_err(|e| anyhow::anyhow!("Failed to run freshclam: {}", e))?;

        if output.status.success() {
            let mut status = self.protection_status.write().await;
            status.last_definition_update = Some(chrono::Utc::now());
            info!("Virus definitions updated successfully");
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Failed to update definitions: {}", error))
        }
    }

    pub async fn set_realtime_protection(&self, enabled: bool) -> Result<()> {
        let mut status = self.protection_status.write().await;
        status.real_time_protection = enabled;
        info!("Real-time protection set to: {}", enabled);
        Ok(())
    }
}
