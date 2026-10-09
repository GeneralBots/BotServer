use super::*;

impl WebhookDelivery {
    pub fn new(webhook: &WebhookEndpoint, payload: &WebhookPayload) -> Self {
        Self {
            id: Uuid::new_v4(),
            webhook_id: webhook.id,
            payload_id: payload.id.clone(),
            event: payload.event.clone(),
            url: webhook.url.clone(),
            status: DeliveryStatus::Pending,
            response_code: None,
            response_body: None,
            attempt: 0,
            created_at: Utc::now(),
            completed_at: None,
            next_retry_at: None,
            error: None,
        }
    }

    pub fn mark_success(&mut self, response_code: u16, response_body: Option<String>) {
        self.status = DeliveryStatus::Success;
        self.response_code = Some(response_code);
        self.response_body = response_body;
        self.completed_at = Some(Utc::now());
        self.attempt += 1;
    }

    pub fn mark_failed(&mut self, error: &str, should_retry: bool, retry_delay: Duration) {
        self.attempt += 1;
        self.error = Some(error.to_string());

        if should_retry {
            self.status = DeliveryStatus::Retrying;
            self.next_retry_at = Some(Utc::now() + retry_delay);
        } else {
            self.status = DeliveryStatus::Failed;
            self.completed_at = Some(Utc::now());
        }
    }
}
