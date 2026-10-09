use super::*;

impl AlertNotification {
    pub fn from_alert(alert: &UsageAlert, prefs: &NotificationPreferences) -> Self {
        Self {
            alert_id: alert.id,
            organization_id: alert.organization_id,
            severity: alert.severity.clone(),
            title: format!(
                "{} Usage Alert: {}",
                alert.severity.emoji(),
                alert.metric.display_name()
            ),
            message: alert.message.clone(),
            metric: alert.metric.display_name().to_string(),
            current_usage: alert.current_usage,
            limit: alert.limit,
            percentage: alert.percentage,
            action_url: Some(format!("/billing/usage?org={}", alert.organization_id)),
            created_at: alert.created_at,
            recipients: prefs.email_recipients.clone(),
        }
    }
}
