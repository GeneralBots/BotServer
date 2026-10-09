use super::*;

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            async_logging: true,
            buffer_size: 1000,
            flush_interval_seconds: 5,
            retention_days: 365,
            min_severity: AuditSeverity::Info,
            categories_enabled: vec![
                AuditEventCategory::Authentication,
                AuditEventCategory::Authorization,
                AuditEventCategory::DataAccess,
                AuditEventCategory::DataModification,
                AuditEventCategory::Administration,
                AuditEventCategory::Security,
                AuditEventCategory::Compliance,
            ],
            tamper_evident: true,
            compress_old_logs: true,
            encrypt_sensitive_data: true,
        }
    }
}
