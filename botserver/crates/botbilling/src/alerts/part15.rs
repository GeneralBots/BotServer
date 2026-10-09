use super::*;

impl std::fmt::Display for NotificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NetworkError(msg) => write!(f, "Network error: {}", msg),
            Self::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            Self::RateLimited => write!(f, "Rate limited"),
            Self::InvalidRecipient(msg) => write!(f, "Invalid recipient: {}", msg),
            Self::DeliveryFailed(msg) => write!(f, "Delivery failed: {}", msg),
        }
    }
}
impl std::error::Error for NotificationError {}
pub(super) fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}
pub(super) fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}
impl UsageMetric {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Messages => "Messages",
            Self::StorageBytes => "Storage",
            Self::ApiCalls => "API Calls",
            Self::Bots => "Bots",
            Self::Users => "Users",
            Self::KbDocuments => "KB Documents",
            Self::Apps => "Apps",
            Self::EmailsPerDay => "Emails Per Day",
            Self::BuildsPerDay => "Builds Per Day",
            Self::UploadsPerHour => "Uploads Per Hour",
        }
    }
}
pub fn create_alert_manager(
    thresholds: Option<AlertThresholds>,
    cooldown_seconds: Option<u64>,
    pool: Option<DbPool>,
) -> AlertManager {
    let mut manager = AlertManager::new();

    if let Some(t) = thresholds {
        manager = manager.with_thresholds(t);
    }

    if let Some(c) = cooldown_seconds {
        manager = manager.with_cooldown(c);
    }

    if let Some(p) = pool {
        manager = manager.with_pool(p);
    }

    manager
}
pub async fn register_default_handlers(manager: &AlertManager) {
    manager
        .register_handler(Arc::new(InAppNotificationHandler::new()))
        .await;
    manager
        .register_handler(Arc::new(WebhookNotificationHandler::new()))
        .await;
}
