use super::*;

impl Default for ApprovalConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_timeout: 3600,
            reminder_interval: 1800,
            max_reminders: 3,
            audit_enabled: true,
            webhook_url: None,
            email_template: None,
            approval_base_url: None,
        }
    }
}
