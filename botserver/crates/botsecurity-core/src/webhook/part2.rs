use super::*;

impl WebhookEndpoint {
    pub fn new(url: &str, events: Vec<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            url: url.to_string(),
            secret: generate_webhook_secret(),
            events,
            status: WebhookStatus::Active,
            created_at: now,
            updated_at: now,
            last_triggered_at: None,
            failure_count: 0,
            allowed_ips: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn is_active(&self) -> bool {
        self.status == WebhookStatus::Active
    }

    pub fn subscribes_to(&self, event: &str) -> bool {
        self.events.iter().any(|e| e == event || e == "*")
    }

    pub fn record_success(&mut self) {
        self.last_triggered_at = Some(Utc::now());
        self.failure_count = 0;
        self.status = WebhookStatus::Active;
        self.updated_at = Utc::now();
    }

    pub fn record_failure(&mut self, max_failures: u32) {
        self.failure_count += 1;
        self.updated_at = Utc::now();

        if self.failure_count >= max_failures {
            self.status = WebhookStatus::Suspended;
            warn!(
                "Webhook {} suspended after {} failures",
                self.id, self.failure_count
            );
        }
    }
}
