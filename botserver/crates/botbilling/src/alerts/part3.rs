use super::*;

impl UsageAlert {
    pub fn new(
        organization_id: Uuid,
        metric: UsageMetric,
        severity: AlertSeverity,
        current_usage: u64,
        limit: u64,
        percentage: f64,
        threshold: f64,
    ) -> Self {
        let severity_clone = severity.clone();
        let message = Self::generate_message(metric, severity, percentage, current_usage, limit);

        Self {
            id: Uuid::new_v4(),
            organization_id,
            metric,
            severity: severity_clone,
            current_usage,
            limit,
            percentage,
            threshold,
            message,
            created_at: Utc::now(),
            acknowledged_at: None,
            acknowledged_by: None,
            notification_sent: false,
            notification_channels: Vec::new(),
        }
    }

    pub(super) fn generate_message(
        metric: UsageMetric,
        severity: AlertSeverity,
        percentage: f64,
        current: u64,
        limit: u64,
    ) -> String {
        let metric_name = metric.display_name();
        let severity_ = match severity {
            AlertSeverity::Warning => "approaching limit",
            AlertSeverity::Critical => "near limit",
            AlertSeverity::Exceeded => "exceeded limit",
        };

        format!(
            "{} {} usage is {} ({:.1}% - {}/{})",
            severity.emoji(),
            metric_name,
            severity_,
            percentage,
            Self::format_value(metric, current),
            Self::format_value(metric, limit)
        )
    }

    pub(super) fn format_value(metric: UsageMetric, value: u64) -> String {
        match metric {
            UsageMetric::StorageBytes => format_bytes(value),
            _ => format_number(value),
        }
    }

    pub fn acknowledge(&mut self, user_id: Uuid) {
        self.acknowledged_at = Some(Utc::now());
        self.acknowledged_by = Some(user_id);
    }

    pub fn is_acknowledged(&self) -> bool {
        self.acknowledged_at.is_some()
    }

    pub fn mark_notified(&mut self, channels: Vec<NotificationChannel>) {
        self.notification_sent = true;
        self.notification_channels = channels;
    }
}
