use super::*;

impl WebhookPayload {
    pub fn new(event: &str, data: serde_json::Value, webhook_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            event: event.to_string(),
            timestamp: Utc::now(),
            data,
            webhook_id,
        }
    }

    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(|e| anyhow!("Failed to serialize payload: {e}"))
    }
}
