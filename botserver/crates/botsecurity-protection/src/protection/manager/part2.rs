use super::*;

impl ToolStatus {
    pub fn not_installed(tool: ProtectionTool) -> Self {
        Self {
            tool,
            installed: false,
            version: None,
            service_running: None,
            last_scan: None,
            last_update: None,
            auto_update: false,
            auto_remediate: false,
            metrics: ToolMetrics::default(),
        }
    }
}
