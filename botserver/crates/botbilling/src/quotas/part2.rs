use super::*;

impl QuotaManager {
    pub(super) fn alert_message(&self, metric: UsageMetric, threshold: f64, current: f64) -> String {
        let metric_name = match metric {
            UsageMetric::Messages => "messages",
            UsageMetric::StorageBytes => "storage",
            UsageMetric::ApiCalls => "API calls",
            UsageMetric::Bots => "bots",
            UsageMetric::Users => "users",
            UsageMetric::KbDocuments => "KB documents",
            UsageMetric::Apps => "apps",
            UsageMetric::EmailsPerDay => "emails per day",
            UsageMetric::BuildsPerDay => "builds per day",
            UsageMetric::UploadsPerHour => "uploads per hour",
        };

        if current >= 100.0 {
            format!("You have reached your {} quota limit.", metric_name)
        } else {
            format!(
                "You have used {}% of your {} quota (threshold: {}%).",
                current as u32, metric_name, threshold as u32
            )
        }
    }
}
