use super::*;

impl Default for DlpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            scan_inbound: true,
            scan_outbound: true,
            block_on_violation: false,
            log_violations: true,
            redact_sensitive_data: true,
            max_content_length: 10 * 1024 * 1024,
        }
    }
}
