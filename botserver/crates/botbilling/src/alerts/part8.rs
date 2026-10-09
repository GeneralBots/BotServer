use super::*;

impl Default for NotificationPreferences {
    fn default() -> Self {
        Self {
            enabled: true,
            channels: vec![NotificationChannel::Email, NotificationChannel::InApp],
            email_recipients: Vec::new(),
            webhook_url: None,
            webhook_secret: None,
            slack_webhook_url: None,
            teams_webhook_url: None,
            sms_numbers: Vec::new(),
            min_severity: AlertSeverity::Warning,
            quiet_hours: None,
            metric_overrides: HashMap::new(),
        }
    }
}
impl NotificationPreferences {
    pub fn should_notify(&self, severity: AlertSeverity) -> bool {
        severity.priority() >= self.min_severity.priority()
    }

    pub fn is_in_quiet_hours(&self) -> bool {
        if let Some(quiet) = &self.quiet_hours {
            quiet.is_active()
        } else {
            false
        }
    }
}
