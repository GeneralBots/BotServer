pub(crate) use anyhow::{anyhow, Result};
pub(crate) use chrono::{DateTime, Duration, Utc};
pub(crate) use rand::Rng;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use sha2::{Digest, Sha256};
pub(crate) use std::collections::{HashMap, HashSet};
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use tracing::info;
pub(crate) use uuid::Uuid;

const API_KEY_PREFIX: &str = "gb_";
const API_KEY_LENGTH: usize = 32;
const API_KEY_HASH_ITERATIONS: u32 = 100_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    pub default_rate_limit_per_minute: u32,
    pub default_rate_limit_per_hour: u32,
    pub default_rate_limit_per_day: u32,
    pub max_keys_per_user: usize,
    pub key_expiry_days: Option<u32>,
    pub allow_key_rotation: bool,
    pub rotation_grace_period_hours: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApiKeyScope {
    Read,
    Write,
    Delete,
    Admin,
    Bot(Uuid),
    Resource(String),
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiKeyStatus {
    Active,
    Expired,
    Revoked,
    Rotating,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub requests_per_day: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub key_hash: String,
    pub key_prefix: String,
    pub scopes: HashSet<ApiKeyScope>,
    pub status: ApiKeyStatus,
    pub rate_limits: RateLimitConfig,
    pub allowed_ips: Option<Vec<String>>,
    pub allowed_origins: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub rotated_from: Option<Uuid>,
    pub rotation_deadline: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub description: Option<String>,
    pub scopes: Vec<ApiKeyScope>,
    pub expires_in_days: Option<u32>,
    pub rate_limits: Option<RateLimitConfig>,
    pub allowed_ips: Option<Vec<String>>,
    pub allowed_origins: Option<Vec<String>>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyResponse {
    pub key: ApiKey,
    pub secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyUsage {
    pub key_id: Uuid,
    pub requests_this_minute: u32,
    pub requests_this_hour: u32,
    pub requests_this_day: u32,
    pub minute_reset_at: DateTime<Utc>,
    pub hour_reset_at: DateTime<Utc>,
    pub day_reset_at: DateTime<Utc>,
    pub total_requests: u64,
    pub last_request_at: Option<DateTime<Utc>>,
}

pub struct ApiKeyManager {
    config: ApiKeyConfig,
    keys: Arc<RwLock<HashMap<Uuid, ApiKey>>>,
    key_hashes: Arc<RwLock<HashMap<String, Uuid>>>,
    usage: Arc<RwLock<HashMap<Uuid, ApiKeyUsage>>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_validate_key() {
        let manager = ApiKeyManager::with_defaults();
        let user_id = Uuid::new_v4();

        let request = CreateApiKeyRequest {
            name: "Test Key".into(),
            description: Some("A test API key".into()),
            scopes: vec![ApiKeyScope::Read, ApiKeyScope::Write],
            expires_in_days: Some(30),
            rate_limits: None,
            allowed_ips: None,
            allowed_origins: None,
            metadata: None,
        };

        let response = manager.create_key(user_id, request).await.expect("Create failed");

        assert!(response.secret.starts_with(API_KEY_PREFIX));
        assert_eq!(response.key.user_id, user_id);
        assert!(response.key.is_valid());

        let validated = manager
            .validate_key(&response.secret)
            .await
            .expect("Validate failed");
        assert!(validated.is_some());
        assert_eq!(validated.as_ref().map(|k| k.id), Some(response.key.id));
    }

    #[tokio::test]
    async fn test_key_scopes() {
        let manager = ApiKeyManager::with_defaults();
        let user_id = Uuid::new_v4();

        let request = CreateApiKeyRequest {
            name: "Limited Key".into(),
            description: None,
            scopes: vec![ApiKeyScope::Read],
            expires_in_days: None,
            rate_limits: None,
            allowed_ips: None,
            allowed_origins: None,
            metadata: None,
        };

        let response = manager.create_key(user_id, request).await.expect("Create failed");

        assert!(response.key.has_scope(&ApiKeyScope::Read));
        assert!(!response.key.has_scope(&ApiKeyScope::Write));
        assert!(!response.key.has_scope(&ApiKeyScope::Admin));
    }

    #[tokio::test]
    async fn test_admin_scope_includes_all() {
        let manager = ApiKeyManager::with_defaults();
        let user_id = Uuid::new_v4();

        let request = CreateApiKeyRequest {
            name: "Admin Key".into(),
            description: None,
            scopes: vec![ApiKeyScope::Admin],
            expires_in_days: None,
            rate_limits: None,
            allowed_ips: None,
            allowed_origins: None,
            metadata: None,
        };

        let response = manager.create_key(user_id, request).await.expect("Create failed");

        assert!(response.key.has_scope(&ApiKeyScope::Read));
        assert!(response.key.has_scope(&ApiKeyScope::Write));
        assert!(response.key.has_scope(&ApiKeyScope::Delete));
        assert!(response.key.has_scope(&ApiKeyScope::Admin));
    }

    #[tokio::test]
    async fn test_revoke_key() {
        let manager = ApiKeyManager::with_defaults();
        let user_id = Uuid::new_v4();

        let request = CreateApiKeyRequest {
            name: "Revokable Key".into(),
            description: None,
            scopes: vec![ApiKeyScope::Read],
            expires_in_days: None,
            rate_limits: None,
            allowed_ips: None,
            allowed_origins: None,
            metadata: None,
        };

        let response = manager.create_key(user_id, request).await.expect("Create failed");
        let key_id = response.key.id;

        manager.revoke_key(key_id).await.expect("Revoke failed");

        let validated = manager
            .validate_key(&response.secret)
            .await
            .expect("Validate failed");
        assert!(validated.is_none());
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let manager = ApiKeyManager::with_defaults();
        let user_id = Uuid::new_v4();

        let request = CreateApiKeyRequest {
            name: "Rate Limited Key".into(),
            description: None,
            scopes: vec![ApiKeyScope::Read],
            expires_in_days: None,
            rate_limits: Some(RateLimitConfig {
                requests_per_minute: 2,
                requests_per_hour: 100,
                requests_per_day: 1000,
            }),
            allowed_ips: None,
            allowed_origins: None,
            metadata: None,
        };

        let response = manager.create_key(user_id, request).await.expect("Create failed");

        let (_, limited1) = manager
            .validate_and_check_rate_limit(&response.secret)
            .await
            .expect("Validate failed");
        assert!(!limited1);

        let (_, limited2) = manager
            .validate_and_check_rate_limit(&response.secret)
            .await
            .expect("Validate failed");
        assert!(!limited2);

        let (_, limited3) = manager
            .validate_and_check_rate_limit(&response.secret)
            .await
            .expect("Validate failed");
        assert!(limited3);
    }

    #[test]
    fn test_ip_cidr_matching() {
        assert!(matches_cidr("192.168.1.100", "192.168.1.0/24"));
        assert!(matches_cidr("192.168.1.1", "192.168.1.0/24"));
        assert!(!matches_cidr("192.168.2.1", "192.168.1.0/24"));
        assert!(matches_cidr("10.0.0.1", "10.0.0.0/8"));
    }

    #[test]
    fn test_extract_api_key_from_header() {
        assert_eq!(
            extract_api_key_from_header("Bearer gb_abc123"),
            Some("gb_abc123")
        );
        assert_eq!(
            extract_api_key_from_header("ApiKey gb_xyz789"),
            Some("gb_xyz789")
        );
        assert_eq!(
            extract_api_key_from_header("gb_direct_key"),
            Some("gb_direct_key")
        );
        assert_eq!(extract_api_key_from_header("Basic abc123"), None);
    }

    #[test]
    fn test_scope_inclusion() {
        assert!(ApiKeyScope::Admin.includes(&ApiKeyScope::Read));
        assert!(ApiKeyScope::Admin.includes(&ApiKeyScope::Write));
        assert!(ApiKeyScope::Write.includes(&ApiKeyScope::Read));
        assert!(!ApiKeyScope::Read.includes(&ApiKeyScope::Write));
    }

    #[test]
    fn test_origin_matching() {
        let key = ApiKey {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            name: "Test".into(),
            description: None,
            key_hash: "hash".into(),
            key_prefix: "gb_abc...".into(),
            scopes: HashSet::new(),
            status: ApiKeyStatus::Active,
            rate_limits: RateLimitConfig::default(),
            allowed_ips: None,
            allowed_origins: Some(vec!["*.example.com".into(), "https://specific.org".into()]),
            created_at: Utc::now(),
            expires_at: None,
            last_used_at: None,
            rotated_from: None,
            rotation_deadline: None,
            metadata: HashMap::new(),
        };

        assert!(key.is_origin_allowed("api.example.com"));
        assert!(key.is_origin_allowed("sub.example.com"));
        assert!(key.is_origin_allowed("https://specific.org"));
        assert!(!key.is_origin_allowed("https://other.org"));
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
