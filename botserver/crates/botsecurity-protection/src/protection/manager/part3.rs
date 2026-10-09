use super::*;

impl Default for ProtectionConfig {
    fn default() -> Self {
        Self {
            enabled_tools: ProtectionTool::all(),
            auto_scan_interval_hours: 24,
            auto_update_interval_hours: 6,
            quarantine_dir: "/var/lib/gb/quarantine".to_string(),
            log_dir: "/var/log/gb/security".to_string(),
        }
    }
}
