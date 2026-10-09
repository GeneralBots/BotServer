use super::*;

impl WebhookManager {
    pub fn new(config: WebhookConfig) -> Self {
        let security = WebhookSecurityManager::new(config.clone());
        Self {
            config,
            security,
            endpoints: Arc::new(RwLock::new(std::collections::HashMap::new())),
            deliveries: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(WebhookConfig::default())
    }

    pub async fn register_endpoint(&self, url: &str, events: Vec<String>) -> Result<WebhookEndpoint> {
        self.security.validate_url(url)?;

        let endpoint = WebhookEndpoint::new(url, events);
        let id = endpoint.id;

        let mut endpoints = self.endpoints.write().await;
        endpoints.insert(id, endpoint.clone());

        info!("Registered webhook endpoint {} for URL {}", id, url);
        Ok(endpoint)
    }

    pub async fn get_endpoint(&self, id: Uuid) -> Option<WebhookEndpoint> {
        let endpoints = self.endpoints.read().await;
        endpoints.get(&id).cloned()
    }

    pub async fn update_endpoint(&self, id: Uuid, updates: WebhookEndpointUpdate) -> Result<WebhookEndpoint> {
        let mut endpoints = self.endpoints.write().await;
        let endpoint = endpoints
            .get_mut(&id)
            .ok_or_else(|| anyhow!("Webhook endpoint not found"))?;

        if let Some(url) = updates.url {
            self.security.validate_url(&url)?;
            endpoint.url = url;
        }

        if let Some(events) = updates.events {
            endpoint.events = events;
        }

        if let Some(status) = updates.status {
            endpoint.status = status;
        }

        endpoint.updated_at = Utc::now();

        Ok(endpoint.clone())
    }

    pub async fn delete_endpoint(&self, id: Uuid) -> bool {
        let mut endpoints = self.endpoints.write().await;
        endpoints.remove(&id).is_some()
    }

    pub async fn get_endpoints_for_event(&self, event: &str) -> Vec<WebhookEndpoint> {
        let endpoints = self.endpoints.read().await;
        endpoints
            .values()
            .filter(|e| e.is_active() && e.subscribes_to(event))
            .cloned()
            .collect()
    }

    pub async fn create_delivery(
        &self,
        endpoint: &WebhookEndpoint,
        event: &str,
        data: serde_json::Value,
    ) -> Result<(WebhookDelivery, String, String, String)> {
        let payload = WebhookPayload::new(event, data, endpoint.id);
        let payload_json = payload.to_json()?;

        self.security.validate_payload_size(payload_json.as_bytes())?;

        let (header_name, signature, timestamp) =
            self.security.create_signature_header(&payload_json, &endpoint.secret);

        let delivery = WebhookDelivery::new(endpoint, &payload);

        let mut deliveries = self.deliveries.write().await;
        deliveries.push(delivery.clone());

        Ok((delivery, payload_json, format!("{header_name}: {signature}"), timestamp))
    }

    pub async fn record_delivery_result(
        &self,
        delivery_id: Uuid,
        success: bool,
        response_code: Option<u16>,
        response_body: Option<String>,
        error: Option<&str>,
    ) -> Result<()> {
        let webhook_id = {
            let mut deliveries = self.deliveries.write().await;
            let delivery = deliveries
                .iter_mut()
                .find(|d| d.id == delivery_id)
                .ok_or_else(|| anyhow!("Delivery not found"))?;

            if success {
                delivery.mark_success(response_code.unwrap_or(200), response_body);
            } else {
                let should_retry = delivery.attempt < self.config.retry_count;
                let retry_delay = Duration::seconds(
                    self.config.retry_delay_seconds as i64 * 2i64.pow(delivery.attempt),
                );
                delivery.mark_failed(error.unwrap_or("Unknown error"), should_retry, retry_delay);
            }

            delivery.webhook_id
        };

        let mut endpoints = self.endpoints.write().await;
        if let Some(endpoint) = endpoints.get_mut(&webhook_id) {
            if success {
                endpoint.record_success();
            } else {
                endpoint.record_failure(self.config.retry_count);
            }
        }

        Ok(())
    }

    pub async fn get_pending_retries(&self) -> Vec<WebhookDelivery> {
        let now = Utc::now();
        let deliveries = self.deliveries.read().await;

        deliveries
            .iter()
            .filter(|d| {
                d.status == DeliveryStatus::Retrying
                    && d.next_retry_at.map(|t| t <= now).unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    pub async fn get_delivery_history(&self, webhook_id: Uuid, limit: usize) -> Vec<WebhookDelivery> {
        let deliveries = self.deliveries.read().await;

        let mut history: Vec<WebhookDelivery> = deliveries
            .iter()
            .filter(|d| d.webhook_id == webhook_id)
            .cloned()
            .collect();

        history.sort_by_key(|b| std::cmp::Reverse(b.created_at));
        history.truncate(limit);
        history
    }

    pub fn security(&self) -> &WebhookSecurityManager {
        &self.security
    }

    pub fn config(&self) -> &WebhookConfig {
        &self.config
    }
}
