use super::*;

impl WebhookNotificationHandler {
    pub fn new() -> Self {
        Self {}
    }
}
impl Default for WebhookNotificationHandler {
    fn default() -> Self {
        Self::new()
    }
}
#[async_trait::async_trait]
impl NotificationHandler for WebhookNotificationHandler {
    fn channel(&self) -> NotificationChannel {
        NotificationChannel::Webhook
    }

    async fn send(&self, notification: &AlertNotification) -> Result<(), NotificationError> {
        tracing::info!(
            "Sending webhook notification for alert {}",
            notification.alert_id
        );

        let webhook_url = std::env::var("BILLING_WEBHOOK_URL").ok();

        let url = match webhook_url {
            Some(url) => url,
            None => {
                tracing::warn!("No webhook URL configured for alert {}", notification.alert_id);
                return Ok(());
            }
        };

        let payload = serde_json::json!({
            "alert_id": notification.alert_id,
            "organization_id": notification.organization_id,
            "alert_type": notification.title,
            "severity": notification.severity.to_string(),
            "message": notification.message,
            "threshold_value": notification.limit,
            "current_value": notification.current_usage,
            "triggered_at": notification.created_at.to_rfc3339(),
            "recipients": notification.recipients,
        });

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("User-Agent", "GeneralBots-Billing-Alerts/1.0")
            .json(&payload)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| NotificationError::DeliveryFailed(format!("Webhook request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NotificationError::DeliveryFailed(
                format!("Webhook returned {}: {}", status, body)
            ));
        }

        tracing::debug!("Webhook notification sent successfully to {}", url);
        Ok(())
    }
}
