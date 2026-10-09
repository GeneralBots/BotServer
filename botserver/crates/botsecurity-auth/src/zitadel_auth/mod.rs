pub(crate) use crate::auth::{AuthConfig, AuthError, AuthenticatedUser, BotAccess, Permission, Role};
pub(crate) use botsecurity_core::VaultConfigProvider;
pub(crate) use botlib::security::create_tls_client;
pub(crate) use anyhow::Result;
pub(crate) use axum::{ body::Body, http::{header, Request}, };
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use tracing::{info, warn};
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZitadelAuthConfig {
    pub issuer_url: String,
    pub api_url: String,
    pub client_id: String,
    #[serde(skip_serializing)]
    pub client_secret: String,
    pub project_id: String,
    pub cache_ttl_secs: u64,
    pub introspect_tokens: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZitadelUser {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub display_name: Option<String>,
    pub roles: Vec<String>,
    pub organization_id: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct CachedUser {
    user: AuthenticatedUser,
    expires_at: i64,
}

pub struct ZitadelAuthProvider {
    config: ZitadelAuthConfig,
    http_client: reqwest::Client,
    user_cache: Arc<RwLock<HashMap<String, CachedUser>>>,
    service_token: Arc<RwLock<Option<ServiceToken>>>,
}

#[derive(Debug, Clone)]
struct ServiceToken {
    access_token: String,
    expires_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zitadel_auth_config_default() {
        let config = ZitadelAuthConfig::default();
        assert_eq!(config.cache_ttl_secs, 300);
        assert!(config.introspect_tokens);
        assert!(!config.is_configured());
    }

    #[test]
    fn test_zitadel_auth_config_is_configured() {
        let config = ZitadelAuthConfig::new(
            "https://auth.example.com",
            "https://api.example.com",
            "client123",
            "secret456",
        );
        assert!(config.is_configured());
    }

    #[test]
    fn test_zitadel_auth_config_builder() {
        let config = ZitadelAuthConfig::new(
            "https://auth.example.com",
            "https://api.example.com",
            "client123",
            "secret456",
        )
        .with_project_id("project789")
        .with_cache_ttl(600)
        .without_introspection();

        assert_eq!(config.issuer_url, "https://auth.example.com");
        assert_eq!(config.api_url, "https://api.example.com");
        assert_eq!(config.client_id, "client123");
        assert_eq!(config.project_id, "project789");
        assert_eq!(config.cache_ttl_secs, 600);
        assert!(!config.introspect_tokens);
    }

    #[test]
    fn test_map_zitadel_role_to_role() {
        assert_eq!(map_zitadel_role_to_role("superadmin"), Role::SuperAdmin);
        assert_eq!(map_zitadel_role_to_role("admin"), Role::Admin);
        assert_eq!(map_zitadel_role_to_role("ADMIN"), Role::Admin);
        assert_eq!(map_zitadel_role_to_role("moderator"), Role::Moderator);
        assert_eq!(map_zitadel_role_to_role("bot_owner"), Role::BotOwner);
        assert_eq!(map_zitadel_role_to_role("bot_operator"), Role::BotOperator);
        assert_eq!(map_zitadel_role_to_role("bot_viewer"), Role::BotViewer);
        assert_eq!(map_zitadel_role_to_role("user"), Role::User);
        assert_eq!(map_zitadel_role_to_role("custom_role"), Role::User);
        assert_eq!(map_zitadel_role_to_role(""), Role::Anonymous);
    }

    #[test]
    fn test_zitadel_user_to_authenticated_user() {
        let zitadel_user = ZitadelUser {
            id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            username: "testuser".to_string(),
            email: Some("test@example.com".to_string()),
            email_verified: true,
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
            display_name: Some("Test User".to_string()),
            roles: vec!["admin".to_string(), "bot_owner".to_string()],
            organization_id: Some("660e8400-e29b-41d4-a716-446655440001".to_string()),
            metadata: HashMap::new(),
        };

        let auth_user = zitadel_user.to_authenticated_user().unwrap();

        assert_eq!(auth_user.username, "testuser");
        assert_eq!(auth_user.email, Some("test@example.com".to_string()));
        assert!(auth_user.has_role(&Role::Admin));
        assert!(auth_user.has_role(&Role::BotOwner));
        assert!(auth_user.is_admin());
    }

    #[test]
    fn test_zitadel_user_invalid_uuid() {
        let zitadel_user = ZitadelUser {
            id: "invalid-uuid".to_string(),
            username: "testuser".to_string(),
            email: None,
            email_verified: false,
            first_name: None,
            last_name: None,
            display_name: None,
            roles: vec![],
            organization_id: None,
            metadata: HashMap::new(),
        };

        assert!(zitadel_user.to_authenticated_user().is_err());
    }

    #[test]
    fn test_base64_url_decode() {
        let encoded = "SGVsbG8gV29ybGQ";
        let decoded = base64_url_decode(encoded).unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), "Hello World");
    }

    #[test]
    fn test_base64_url_decode_with_special_chars() {
        let encoded = "PDw_Pz4-";
        let result = base64_url_decode(encoded);
        assert!(result.is_ok());
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

use part2::*;
use part5::*;
