pub(crate) use anyhow::{anyhow, Result};
pub(crate) use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
pub(crate) use chrono::{DateTime, Duration, Utc};
pub(crate) use hmac::{Hmac, Mac};
pub(crate) use rand::Rng;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use sha2::Sha256;
pub(crate) use std::collections::HashSet;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use tracing::{info, warn};
pub(crate) use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

const DEFAULT_TIMESTAMP_TOLERANCE_SECONDS: i64 = 300;
const DEFAULT_REPLAY_WINDOW_SECONDS: i64 = 600;
const SIGNATURE_HEADER: &str = "X-Webhook-Signature";
const SIGNATURE_VERSION: &str = "v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub enabled: bool,
    pub timestamp_tolerance_seconds: i64,
    pub replay_window_seconds: i64,
    pub require_https: bool,
    pub max_payload_size: usize,
    pub retry_count: u32,
    pub retry_delay_seconds: u64,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebhookStatus {
    Active,
    Inactive,
    Failed,
    Suspended,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEndpoint {
    pub id: Uuid,
    pub url: String,
    pub secret: String,
    pub events: Vec<String>,
    pub status: WebhookStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_triggered_at: Option<DateTime<Utc>>,
    pub failure_count: u32,
    pub allowed_ips: Option<Vec<String>>,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub id: String,
    pub event: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
    pub webhook_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDelivery {
    pub id: Uuid,
    pub webhook_id: Uuid,
    pub payload_id: String,
    pub event: String,
    pub url: String,
    pub status: DeliveryStatus,
    pub response_code: Option<u16>,
    pub response_body: Option<String>,
    pub attempt: u32,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryStatus {
    Pending,
    Success,
    Failed,
    Retrying,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignatureValidation {
    Valid,
    Missing,
    Invalid,
    Expired,
    Replayed,
}

pub struct WebhookSecurityManager {
    config: WebhookConfig,
    used_signatures: Arc<RwLock<HashSet<String>>>,
    signature_timestamps: Arc<RwLock<std::collections::HashMap<String, DateTime<Utc>>>>,
}

pub struct WebhookManager {
    config: WebhookConfig,
    security: WebhookSecurityManager,
    endpoints: Arc<RwLock<std::collections::HashMap<Uuid, WebhookEndpoint>>>,
    deliveries: Arc<RwLock<Vec<WebhookDelivery>>>,
}

#[derive(Debug, Clone, Default)]
pub struct WebhookEndpointUpdate {
    pub url: Option<String>,
    pub events: Option<Vec<String>>,
    pub status: Option<WebhookStatus>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_endpoint_creation() {
        let endpoint = WebhookEndpoint::new(
            "https://example.com/webhook",
            vec!["user.created".into(), "user.updated".into()],
        );

        assert!(endpoint.is_active());
        assert!(endpoint.secret.starts_with("whsec_"));
        assert!(endpoint.subscribes_to("user.created"));
        assert!(!endpoint.subscribes_to("user.deleted"));
    }

    #[test]
    fn test_webhook_wildcard_subscription() {
        let endpoint = WebhookEndpoint::new("https://example.com/webhook", vec!["*".into()]);

        assert!(endpoint.subscribes_to("user.created"));
        assert!(endpoint.subscribes_to("order.completed"));
        assert!(endpoint.subscribes_to("any.event"));
    }

    #[test]
    fn test_signature_generation() {
        let manager = WebhookSecurityManager::with_defaults();
        let payload = r#"{"event":"test"}"#;
        let secret = "test_secret";
        let timestamp = Utc::now();

        let signature = manager.sign_payload(payload, secret, timestamp);

        assert!(signature.starts_with("v1="));
        assert_eq!(signature.len(), 3 + 64);
    }

    #[tokio::test]
    async fn test_signature_verification() {
        let manager = WebhookSecurityManager::with_defaults();
        let payload = r#"{"event":"test"}"#;
        let secret = "test_secret";
        let timestamp = Utc::now();

        let signature = manager.sign_payload(payload, secret, timestamp);
        let timestamp_str = timestamp.timestamp().to_string();

        let result = manager
            .verify_signature(payload, &signature, &timestamp_str, secret)
            .await;

        assert!(result.is_valid());
    }

    #[tokio::test]
    async fn test_replay_protection() {
        let manager = WebhookSecurityManager::with_defaults();
        let payload = r#"{"event":"test"}"#;
        let secret = "test_secret";
        let timestamp = Utc::now();

        let signature = manager.sign_payload(payload, secret, timestamp);
        let timestamp_str = timestamp.timestamp().to_string();

        let result1 = manager
            .verify_signature(payload, &signature, &timestamp_str, secret)
            .await;
        assert!(result1.is_valid());

        let result2 = manager
            .verify_signature(payload, &signature, &timestamp_str, secret)
            .await;
        assert_eq!(result2, SignatureValidation::Replayed);
    }

    #[tokio::test]
    async fn test_expired_timestamp() {
        let mut config = WebhookConfig::default();
        config.timestamp_tolerance_seconds = 60;
        let manager = WebhookSecurityManager::new(config);

        let payload = r#"{"event":"test"}"#;
        let secret = "test_secret";
        let old_timestamp = Utc::now() - Duration::seconds(120);

        let signature = manager.sign_payload(payload, secret, old_timestamp);
        let timestamp_str = old_timestamp.timestamp().to_string();

        let result = manager
            .verify_signature(payload, &signature, &timestamp_str, secret)
            .await;

        assert_eq!(result, SignatureValidation::Expired);
    }

    #[test]
    fn test_url_validation() {
        let manager = WebhookSecurityManager::with_defaults();

        assert!(manager.validate_url("https://example.com/webhook").is_ok());
        assert!(manager.validate_url("http://example.com/webhook").is_err());
        assert!(manager.validate_url("https://localhost/webhook").is_err());
        assert!(manager.validate_url("https://127.0.0.1/webhook").is_err());
        assert!(manager.validate_url("").is_err());
    }

    #[test]
    fn test_url_validation_no_https_required() {
        let mut config = WebhookConfig::default();
        config.require_https = false;
        let manager = WebhookSecurityManager::new(config);

        assert!(manager.validate_url("http://example.com/webhook").is_ok());
        assert!(manager.validate_url("https://example.com/webhook").is_ok());
    }

    #[test]
    fn test_payload_size_validation() {
        let mut config = WebhookConfig::default();
        config.max_payload_size = 100;
        let manager = WebhookSecurityManager::new(config);

        let small_payload = vec![0u8; 50];
        assert!(manager.validate_payload_size(&small_payload).is_ok());

        let large_payload = vec![0u8; 200];
        assert!(manager.validate_payload_size(&large_payload).is_err());
    }

    #[test]
    fn test_webhook_payload_creation() {
        let webhook_id = Uuid::new_v4();
        let payload = WebhookPayload::new(
            "user.created",
            serde_json::json!({"user_id": "123"}),
            webhook_id,
        );

        assert_eq!(payload.event, "user.created");
        assert_eq!(payload.webhook_id, webhook_id);

        let json = payload.to_json().expect("Serialization failed");
        assert!(json.contains("user.created"));
    }

    #[test]
    fn test_delivery_status_transitions() {
        let endpoint = WebhookEndpoint::new("https://example.com", vec!["test".into()]);
        let payload = WebhookPayload::new("test", serde_json::json!({}), endpoint.id);
        let mut delivery = WebhookDelivery::new(&endpoint, &payload);

        assert_eq!(delivery.status, DeliveryStatus::Pending);
        assert_eq!(delivery.attempt, 0);

        delivery.mark_success(200, Some("OK".into()));
        assert_eq!(delivery.status, DeliveryStatus::Success);
        assert_eq!(delivery.attempt, 1);
        assert!(delivery.completed_at.is_some());
    }

    #[test]
    fn test_delivery_retry_logic() {
        let endpoint = WebhookEndpoint::new("https://example.com", vec!["test".into()]);
        let payload = WebhookPayload::new("test", serde_json::json!({}), endpoint.id);
        let mut delivery = WebhookDelivery::new(&endpoint, &payload);

        delivery.mark_failed("Connection error", true, Duration::seconds(60));
        assert_eq!(delivery.status, DeliveryStatus::Retrying);
        assert!(delivery.next_retry_at.is_some());
        assert!(delivery.completed_at.is_none());

        delivery.mark_failed("Connection error", false, Duration::seconds(60));
        assert_eq!(delivery.status, DeliveryStatus::Failed);
        assert!(delivery.completed_at.is_some());
    }

    #[test]
    fn test_endpoint_failure_tracking() {
        let mut endpoint = WebhookEndpoint::new("https://example.com", vec!["test".into()]);

        endpoint.record_failure(3);
        assert_eq!(endpoint.failure_count, 1);
        assert_eq!(endpoint.status, WebhookStatus::Active);

        endpoint.record_failure(3);
        endpoint.record_failure(3);
        assert_eq!(endpoint.failure_count, 3);
        assert_eq!(endpoint.status, WebhookStatus::Suspended);

        endpoint.record_success();
        assert_eq!(endpoint.failure_count, 0);
        assert_eq!(endpoint.status, WebhookStatus::Active);
    }

    #[tokio::test]
    async fn test_webhook_manager_registration() {
        let manager = WebhookManager::with_defaults();

        let endpoint = manager
            .register_endpoint("https://example.com/webhook", vec!["user.created".into()])
            .await
            .expect("Registration failed");

        assert!(endpoint.is_active());

        let retrieved = manager.get_endpoint(endpoint.id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().url, "https://example.com/webhook");
    }

    #[tokio::test]
    async fn test_webhook_manager_event_filtering() {
        let manager = WebhookManager::with_defaults();

        manager
            .register_endpoint("https://a.com/webhook", vec!["user.created".into()])
            .await
            .expect("Registration failed");

        manager
            .register_endpoint("https://b.com/webhook", vec!["order.created".into()])
            .await
            .expect("Registration failed");

        let user_endpoints = manager.get_endpoints_for_event("user.created").await;
        assert_eq!(user_endpoints.len(), 1);
        assert_eq!(user_endpoints[0].url, "https://a.com/webhook");
    }

    #[test]
    fn test_parse_signature_header() {
        let (version, sig) = parse_signature_header("v1=abc123").expect("Parse failed");
        assert_eq!(version, "v1");
        assert_eq!(sig, "abc123");

        assert!(parse_signature_header("invalid").is_none());
    }

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("abc123", "abc123"));
        assert!(!constant_time_compare("abc123", "abc124"));
        assert!(!constant_time_compare("abc", "abcd"));
    }

    #[tokio::test]
    async fn test_signature_cleanup() {
        let mut config = WebhookConfig::default();
        config.replay_window_seconds = 0;
        let manager = WebhookSecurityManager::new(config);

        let payload = r#"{"event":"test"}"#;
        let secret = "test_secret";
        let timestamp = Utc::now();
        let signature = manager.sign_payload(payload, secret, timestamp);
        let timestamp_str = timestamp.timestamp().to_string();

        manager
            .verify_signature(payload, &signature, &timestamp_str, secret)
            .await;

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let cleaned = manager.cleanup_old_signatures().await;
        assert!(cleaned >= 1);
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;
mod part8;

pub use part8::*;
