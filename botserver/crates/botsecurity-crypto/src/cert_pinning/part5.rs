use super::*;

impl CertPinningManager {
    pub fn get_stats(&self) -> Result<PinningStats> {
        let config = self
            .config
            .read()
            .map_err(|_| anyhow!("Failed to acquire read lock"))?;

        let mut total_pins = 0;
        let mut backup_pins = 0;
        let mut expiring_soon = 0;

        let now = chrono::Utc::now();
        let soon = now + chrono::Duration::days(30);

        for pins in config.pins.values() {
            for pin in pins {
                total_pins += 1;
                if pin.is_backup {
                    backup_pins += 1;
                }
                if let Some(expires) = pin.expires_at {
                    if expires < soon {
                        expiring_soon += 1;
                    }
                }
            }
        }

        Ok(PinningStats {
            enabled: config.enabled,
            total_hosts: config.pins.len(),
            total_pins,
            backup_pins,
            expiring_soon,
            report_only: config.report_only,
        })
    }
}
