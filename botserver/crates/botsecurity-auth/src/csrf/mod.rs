pub(crate) use anyhow::{anyhow, Result};
pub(crate) use axum::{ body::Body, http::{header, Request, StatusCode}, middleware::Next, response::{IntoResponse, Response}, Json, };
pub(crate) use base64::{engine::general_purpose::URL_SAFE_NO_PAD as BASE64, Engine};
pub(crate) use chrono::{DateTime, Duration, Utc};
pub(crate) use hmac::{Hmac, Mac};
pub(crate) use rand::Rng;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use sha2::Sha256;
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use tracing::warn;

const TOKEN_LENGTH: usize = 32;
const DEFAULT_TOKEN_EXPIRY_MINUTES: i64 = 60;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsrfConfig {
    pub enabled: bool,
    pub token_expiry_minutes: i64,
    pub cookie_name: String,
    pub header_name: String,
    pub form_field_name: String,
    pub cookie_secure: bool,
    pub cookie_same_site: SameSite,
    pub exempt_paths: Vec<String>,
    pub exempt_methods: Vec<String>,
    pub double_submit_cookie: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsrfToken {
    pub token: String,
    pub session_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsrfValidationResult {
    Valid,
    Missing,
    Invalid,
    Expired,
    SessionMismatch,
}

pub struct CsrfManager {
    config: CsrfConfig,
    secret: Vec<u8>,
    tokens: Arc<RwLock<HashMap<String, CsrfToken>>>,
}

#[derive(Clone)]
pub struct CsrfLayer {
    manager: Arc<CsrfManager>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manager() -> CsrfManager {
        CsrfManager::from_secret("this-is-a-very-long-secret-key-for-testing-csrf")
            .expect("Failed to create manager")
    }

    #[tokio::test]
    async fn test_generate_and_validate_token() {
        let manager = create_test_manager();

        let token = manager.generate_token().await;
        assert!(!token.token.is_empty());
        assert!(token.is_valid());

        let result = manager.validate_token(&token.token).await;
        assert!(result.is_valid());
    }

    #[tokio::test]
    async fn test_token_with_session() {
        let manager = create_test_manager();
        let session_id = "test-session-123";

        let token = manager.generate_token_with_session(session_id).await;
        assert_eq!(token.session_id, Some(session_id.to_string()));

        let result = manager.validate_token_with_session(&token.token, session_id).await;
        assert!(result.is_valid());

        let result = manager.validate_token_with_session(&token.token, "wrong-session").await;
        assert_eq!(result, CsrfValidationResult::SessionMismatch);
    }

    #[test]
    fn test_signed_token() {
        let manager = create_test_manager();

        let signed = manager.generate_signed_token();
        assert!(manager.verify_signed_token(&signed));
        assert!(!manager.verify_signed_token("invalid.token.here"));
    }

    #[test]
    fn test_double_submit_validation() {
        let manager = create_test_manager();

        let token = manager.generate_signed_token();
        let result = manager.validate_double_submit(&token, &token);
        assert!(result.is_valid());

        let result = manager.validate_double_submit(&token, "different-token");
        assert_eq!(result, CsrfValidationResult::Invalid);
    }

    #[tokio::test]
    async fn test_revoke_token() {
        let manager = create_test_manager();

        let token = manager.generate_token().await;
        assert!(manager.validate_token(&token.token).await.is_valid());

        manager.revoke_token(&token.token).await;
        assert_eq!(
            manager.validate_token(&token.token).await,
            CsrfValidationResult::Invalid
        );
    }

    #[test]
    fn test_exempt_paths() {
        let manager = create_test_manager();

        assert!(manager.is_exempt_path("/api/health"));
        assert!(manager.is_exempt_path("/api/version"));
        assert!(!manager.is_exempt_path("/api/users"));
    }

    #[test]
    fn test_exempt_methods() {
        let manager = create_test_manager();

        assert!(manager.is_exempt_method("GET"));
        assert!(manager.is_exempt_method("HEAD"));
        assert!(manager.is_exempt_method("OPTIONS"));
        assert!(!manager.is_exempt_method("POST"));
        assert!(!manager.is_exempt_method("PUT"));
        assert!(!manager.is_exempt_method("DELETE"));
    }

    #[test]
    fn test_extract_csrf_from_cookie() {
        let cookie = "session=abc123; csrf_token=xyz789; other=value";
        let token = extract_csrf_from_cookie(cookie, "csrf_token");
        assert_eq!(token, Some("xyz789".to_string()));

        let token = extract_csrf_from_cookie(cookie, "nonexistent");
        assert_eq!(token, None);
    }

    #[test]
    fn test_extract_csrf_from_form() {
        let body = "username=test&_csrf=abc123&password=secret";
        let token = extract_csrf_from_form(body, "_csrf");
        assert_eq!(token, Some("abc123".to_string()));

        let token = extract_csrf_from_form(body, "nonexistent");
        assert_eq!(token, None);
    }

    #[test]
    fn test_build_cookie() {
        let manager = create_test_manager();
        let cookie = manager.build_cookie("test-token");

        assert!(cookie.contains("csrf_token=test-token"));
        assert!(cookie.contains("Path=/"));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Strict"));
    }

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("abc123", "abc123"));
        assert!(!constant_time_compare("abc123", "abc124"));
        assert!(!constant_time_compare("abc", "abcd"));
        assert!(!constant_time_compare("", "a"));
    }

    #[test]
    fn test_same_site_as_str() {
        assert_eq!(SameSite::Strict.as_str(), "Strict");
        assert_eq!(SameSite::Lax.as_str(), "Lax");
        assert_eq!(SameSite::None.as_str(), "None");
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let mut config = CsrfConfig::default();
        config.token_expiry_minutes = 0;
        let manager = CsrfManager::new(config, b"this-is-a-very-long-secret-key-for-testing")
            .expect("Failed to create manager");

        manager.generate_token().await;
        manager.generate_token().await;

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let cleaned = manager.cleanup_expired().await;
        assert_eq!(cleaned, 2);
    }

    #[test]
    fn test_csrf_validation_result_messages() {
        assert_eq!(CsrfValidationResult::Valid.error_message(), "Valid");
        assert_eq!(CsrfValidationResult::Missing.error_message(), "CSRF token missing");
        assert_eq!(CsrfValidationResult::Invalid.error_message(), "CSRF token invalid");
        assert_eq!(CsrfValidationResult::Expired.error_message(), "CSRF token expired");
        assert_eq!(
            CsrfValidationResult::SessionMismatch.error_message(),
            "CSRF token session mismatch"
        );
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;

pub use part5::*;
pub use part6::*;
