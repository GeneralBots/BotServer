use super::*;

impl WebhookSecurityManager {
    pub fn new(config: WebhookConfig) -> Self {
        Self {
            config,
            used_signatures: Arc::new(RwLock::new(HashSet::new())),
            signature_timestamps: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(WebhookConfig::default())
    }

    pub fn sign_payload(&self, payload: &str, secret: &str, timestamp: DateTime<Utc>) -> String {
        let timestamp_str = timestamp.timestamp().to_string();
        let signed_payload = format!("{}.{}", timestamp_str, payload);

        // #1368 — HmacSha256 accepts any key length, so a construction error
        // means the crypto backend is unusable. Fail closed with an empty
        // signature (verification rejects it) instead of aborting the process.
        let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
            Ok(mac) => mac,
            Err(e) => {
                log::error!("HMAC key error: {e}");
                return format!("{SIGNATURE_VERSION}=");
            }
        };
        mac.update(signed_payload.as_bytes());
        let result = mac.finalize();

        let signature = hex::encode(result.into_bytes());
        format!("{}={}", SIGNATURE_VERSION, signature)
    }

    pub fn create_signature_header(
        &self,
        payload: &str,
        secret: &str,
    ) -> (String, String, String) {
        let timestamp = Utc::now();
        let timestamp_str = timestamp.timestamp().to_string();
        let signature = self.sign_payload(payload, secret, timestamp);

        (
            SIGNATURE_HEADER.to_string(),
            signature,
            timestamp_str,
        )
    }

    pub async fn verify_signature(
        &self,
        payload: &str,
        signature_header: &str,
        timestamp_header: &str,
        secret: &str,
    ) -> SignatureValidation {
        if signature_header.is_empty() {
            return SignatureValidation::Missing;
        }

        let timestamp: i64 = match timestamp_header.parse() {
            Ok(t) => t,
            Err(_) => return SignatureValidation::Invalid,
        };

        let request_time = match DateTime::from_timestamp(timestamp, 0) {
            Some(t) => t,
            None => return SignatureValidation::Invalid,
        };

        let now = Utc::now();
        let tolerance = Duration::seconds(self.config.timestamp_tolerance_seconds);

        if now - request_time > tolerance || request_time - now > tolerance {
            return SignatureValidation::Expired;
        }

        let expected_signature = self.sign_payload(payload, secret, request_time);

        if !constant_time_compare(signature_header, &expected_signature) {
            return SignatureValidation::Invalid;
        }

        let signature_key = format!("{}:{}", signature_header, timestamp_header);

        {
            let signatures = self.used_signatures.read().await;
            if signatures.contains(&signature_key) {
                return SignatureValidation::Replayed;
            }
        }

        {
            let mut signatures = self.used_signatures.write().await;
            signatures.insert(signature_key.clone());
        }

        {
            let mut timestamps = self.signature_timestamps.write().await;
            timestamps.insert(signature_key, Utc::now());
        }

        SignatureValidation::Valid
    }

    pub async fn cleanup_old_signatures(&self) -> usize {
        let cutoff = Utc::now() - Duration::seconds(self.config.replay_window_seconds);

        let expired_keys: Vec<String> = {
            let timestamps = self.signature_timestamps.read().await;
            timestamps
                .iter()
                .filter(|(_, &time)| time < cutoff)
                .map(|(key, _)| key.clone())
                .collect()
        };

        let count = expired_keys.len();

        if count > 0 {
            let mut signatures = self.used_signatures.write().await;
            let mut timestamps = self.signature_timestamps.write().await;

            for key in expired_keys {
                signatures.remove(&key);
                timestamps.remove(&key);
            }

            info!("Cleaned up {} expired webhook signatures", count);
        }

        count
    }

    pub fn validate_url(&self, url: &str) -> Result<()> {
        if url.is_empty() {
            return Err(anyhow!("Webhook URL cannot be empty"));
        }

        if self.config.require_https && !url.starts_with("https://") {
            return Err(anyhow!("Webhook URL must use HTTPS"));
        }

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(anyhow!("Invalid webhook URL scheme"));
        }

        let forbidden_hosts = ["localhost", "127.0.0.1", "0.0.0.0", "::1", "[::1]"];
        for host in forbidden_hosts {
            if url.contains(host) {
                return Err(anyhow!("Webhook URL cannot point to localhost"));
            }
        }

        if url.contains("169.254.") || url.contains("10.") || url.contains("192.168.") {
            return Err(anyhow!("Webhook URL cannot point to private IP addresses"));
        }

        Ok(())
    }

    pub fn validate_payload_size(&self, payload: &[u8]) -> Result<()> {
        if payload.len() > self.config.max_payload_size {
            return Err(anyhow!(
                "Payload size {} exceeds maximum {}",
                payload.len(),
                self.config.max_payload_size
            ));
        }
        Ok(())
    }

    pub fn config(&self) -> &WebhookConfig {
        &self.config
    }
}
