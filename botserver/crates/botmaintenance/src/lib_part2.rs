use super::*;

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            organization_id: Uuid::nil(),
            retention_days: DEFAULT_RETENTION_DAYS,
            enabled_categories: CleanupCategory::default_enabled(),
            auto_cleanup_enabled: false,
            auto_cleanup_schedule: None,
            storage_alert_threshold_percent: 80,
        }
    }
}
