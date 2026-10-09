use super::*;

impl InAppNotificationHandler {
    pub fn new() -> Self {
        Self { broadcast: None }
    }

    pub fn with_broadcast(
        broadcast: tokio::sync::broadcast::Sender<BillingAlertNotification>,
    ) -> Self {
        Self {
            broadcast: Some(broadcast),
        }
    }
}
impl Default for InAppNotificationHandler {
    fn default() -> Self {
        Self::new()
    }
}
#[async_trait::async_trait]
impl NotificationHandler for InAppNotificationHandler {
    fn channel(&self) -> NotificationChannel {
        NotificationChannel::InApp
    }

    async fn send(&self, notification: &AlertNotification) -> Result<(), NotificationError> {
        tracing::info!(
            "Creating in-app notification for alert {} org {}",
            notification.alert_id,
            notification.organization_id
        );

        let ws_notification = BillingAlertNotification {
            alert_id: notification.alert_id,
            organization_id: notification.organization_id,
            severity: notification.severity.to_string(),
            alert_type: notification.title.clone(),
            title: notification.title.clone(),
            message: notification.message.clone(),
            metric: notification.metric.clone(),
            percentage: notification.percentage,
            triggered_at: notification.created_at,
        };

        if let Some(ref broadcast) = self.broadcast {
            match broadcast.send(ws_notification.clone()) {
                Ok(receivers) => {
                    tracing::info!(
                        "Billing alert {} broadcast to {} WebSocket receivers",
                        notification.alert_id,
                        receivers
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "No active WebSocket receivers for billing alert {}: {}",
                        notification.alert_id,
                        e
                    );
                }
            }
        } else {
            tracing::debug!(
                "No broadcast channel configured, alert {} will be delivered via polling",
                notification.alert_id
            );
        }

        Ok(())
    }
}
