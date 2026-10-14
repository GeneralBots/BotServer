use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::net::{IpAddr, ToSocketAddrs};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub struct WebhookEvent {
    pub id: Uuid,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub signature: String,
    pub timestamp: DateTime<Utc>,
    pub delivery_status: DeliveryStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeliveryStatus {
    Pending,
    Delivered,
    Failed(String),
}

impl WebhookEvent {
    pub fn new(event: &str, payload: serde_json::Value, secret: &str) -> Self {
        let timestamp = Utc::now();
        let signature = Self::compute_signature(&payload, secret, &timestamp);

        Self {
            id: Uuid::new_v4(),
            event_type: event.to_string(),
            payload,
            signature,
            timestamp,
            delivery_status: DeliveryStatus::Pending,
        }
    }

    pub fn verify_signature(&self, secret: &str) -> bool {
        let expected = Self::compute_signature(&self.payload, secret, &self.timestamp);
        expected == self.signature
    }

    fn compute_signature(payload: &serde_json::Value, secret: &str, timestamp: &DateTime<Utc>) -> String {
        let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
            Ok(m) => m,
            Err(_) => return String::new(),
        };

        let data = format!("{}{}", timestamp.timestamp(), payload);
        mac.update(data.as_bytes());
        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }

    fn is_safe_webhook_url(url: &str) -> bool {
        let parsed = match url::Url::parse(url) {
            Ok(u) => u,
            Err(_) => return false,
        };
        let host = match parsed.host_str() {
            Some(h) => h,
            None => return false,
        };
        if host == "localhost" || host == "127.0.0.1" || host == "::1" || host == "[::1]" {
            return false;
        }
        if host.ends_with(".local") || host.ends_with(".internal") {
            return false;
        }
        if let Ok(addrs) = (host, 0).to_socket_addrs() {
            for addr in addrs {
                match addr.ip() {
                    IpAddr::V4(v4) => {
                        if v4.is_loopback() || v4.is_private() || v4.is_link_local() || v4.is_unspecified() {
                            return false;
                        }
                    }
                    IpAddr::V6(v6) => {
                        if v6.is_loopback() || v6.is_unspecified() {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    pub fn dispatch(&self, url: &str) -> Result<(), String> {
        if !Self::is_safe_webhook_url(url) {
            return Err("Blocked webhook to unsafe URL".to_string());
        }
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let body = serde_json::json!({
            "event": self.event_type,
            "id": self.id.to_string(),
            "timestamp": self.timestamp.to_rfc3339(),
            "payload": self.payload,
            "signature": self.signature,
        });

        let resp = client
            .post(url)
            .header("Content-Type", "application/json")
            .header("X-Webhook-Signature", &self.signature)
            .header("X-Webhook-Event", &self.event_type)
            .json(&body)
            .send()
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            Err(format!("Webhook delivery failed ({}): {}", status, text))
        }
    }
}

pub struct WebhookDispatcher {
    subscribers: Vec<WebhookSubscriber>,
}

#[derive(Debug, Clone)]
pub struct WebhookSubscriber {
    pub id: Uuid,
    pub url: String,
    pub events: Vec<String>,
    pub secret: String,
    pub retry_count: u32,
}

impl WebhookDispatcher {
    pub fn new() -> Self {
        Self {
            subscribers: Vec::new(),
        }
    }

    pub fn subscribe(&mut self, url: &str, events: &[&str], secret: &str) -> Uuid {
        let sub = WebhookSubscriber {
            id: Uuid::new_v4(),
            url: url.to_string(),
            events: events.iter().map(|e| e.to_string()).collect(),
            secret: secret.to_string(),
            retry_count: 3,
        };
        let id = sub.id;
        self.subscribers.push(sub);
        id
    }

    pub fn unsubscribe(&mut self, id: &Uuid) -> bool {
        let before = self.subscribers.len();
        self.subscribers.retain(|s| &s.id != id);
        self.subscribers.len() < before
    }

    pub fn dispatch_event(&self, event: &WebhookEvent) -> Vec<(String, Result<(), String>)> {
        let mut results = Vec::new();
        for sub in &self.subscribers {
            if !sub.events.contains(&event.event_type) {
                continue;
            }

            let current_event = event.clone();
            let result = current_event.dispatch(&sub.url);

            if result.is_err() && sub.retry_count > 0 {
                for attempt in 1..=sub.retry_count {
                    std::thread::sleep(std::time::Duration::from_secs(2u64.pow(attempt)));
                    let retry_event = event.clone();
                    let retry_result = retry_event.dispatch(&sub.url);
                    if retry_result.is_ok() {
                        results.push((sub.url.clone(), Ok(())));
                        break;
                    }
                    if attempt == sub.retry_count {
                        results.push((sub.url.clone(), retry_result));
                    }
                }
            } else {
                results.push((sub.url.clone(), result));
            }
        }
        results
    }

    pub fn subscribers_for_event(&self, event_type: &str) -> Vec<&WebhookSubscriber> {
        self.subscribers.iter()
            .filter(|s| s.events.contains(&event_type.to_string()))
            .collect()
    }
}

impl Default for WebhookDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_event_new() {
        let payload = serde_json::json!({"contact_id": "123"});
        let event = WebhookEvent::new("contact.created", payload.clone(), "secret123");
        assert_eq!(event.event_type, "contact.created");
        assert_eq!(event.payload, payload);
        assert!(!event.signature.is_empty());
    }

    #[test]
    fn test_signature_verification() {
        let payload = serde_json::json!({"test": true});
        let event = WebhookEvent::new("test.event", payload, "secret123");
        assert!(event.verify_signature("secret123"));
        assert!(!event.verify_signature("wrong_secret"));
    }

    #[test]
    fn test_subscribe_unsubscribe() {
        let mut disp = WebhookDispatcher::new();
        let id = disp.subscribe("https://ex.com/hook", &["contact.created"], "sec");
        assert_eq!(disp.subscribers.len(), 1);
        assert!(disp.unsubscribe(&id));
        assert_eq!(disp.subscribers.len(), 0);
    }

    #[test]
    fn test_subscribe_filter() {
        let mut disp = WebhookDispatcher::new();
        disp.subscribe("https://ex.com/hook", &["contact.created"], "sec");
        let subs = disp.subscribers_for_event("contact.created");
        assert_eq!(subs.len(), 1);
        let subs = disp.subscribers_for_event("ticket.updated");
        assert_eq!(subs.len(), 0);
    }
}
