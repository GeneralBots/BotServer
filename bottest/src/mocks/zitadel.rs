use super::{new_expectation_store, ExpectationStore};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use wiremock::matchers::{body_string_contains, header, method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub struct MockZitadel {
    server: MockServer,
    port: u16,
    expectations: ExpectationStore,
    users: Arc<Mutex<HashMap<String, TestUser>>>,
    tokens: Arc<Mutex<HashMap<String, TokenInfo>>>,
    issuer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestUser {
    pub id: String,
    pub email: String,
    pub name: String,
    pub password: String,
    pub roles: Vec<String>,
    pub metadata: HashMap<String, String>,
}

impl Default for TestUser {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            password: "password123".to_string(),
            roles: vec!["user".to_string()],
            metadata: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct TokenInfo {
    active: bool,
}

#[derive(Serialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id_token: Option<String>,
    scope: String,
}

#[derive(Serialize)]
struct OIDCDiscovery {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    userinfo_endpoint: String,
    introspection_endpoint: String,
    revocation_endpoint: String,
    jwks_uri: String,
    response_types_supported: Vec<String>,
    subject_types_supported: Vec<String>,
    id_token_signing_alg_values_supported: Vec<String>,
    scopes_supported: Vec<String>,
    token_endpoint_auth_methods_supported: Vec<String>,
    claims_supported: Vec<String>,
}

#[derive(Serialize)]
struct IntrospectionResponse {
    active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    iat: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sub: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    aud: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    iss: Option<String>,
}

#[derive(Serialize)]
struct UserInfoResponse {
    sub: String,
    email: String,
    email_verified: bool,
    name: String,
    preferred_username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    roles: Option<Vec<String>>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    error_description: String,
}

mod part2;
mod part3;

#[cfg(test)]
mod tests {
    use super::part3::base64_url_encode;
    use super::*;

    #[test]
    fn test_test_user_default() {
        let user = TestUser::default();
        assert!(!user.id.is_empty());
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.roles, vec!["user"]);
    }

    #[test]
    fn test_base64_url_encode() {
        let encoded = base64_url_encode("hello");
        assert!(!encoded.contains('+'));
        assert!(!encoded.contains('/'));
        assert!(!encoded.contains('='));
    }

    #[test]
    fn test_token_response_serialization() {
        let response = TokenResponse {
            access_token: "test_token".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: Some("refresh".to_string()),
            id_token: None,
            scope: "openid".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("access_token"));
        assert!(json.contains("Bearer"));
        assert!(json.contains("refresh_token"));
        assert!(!json.contains("id_token"));
    }

    #[test]
    fn test_introspection_response_active() {
        let response = IntrospectionResponse {
            active: true,
            scope: Some("openid".to_string()),
            client_id: Some("client".to_string()),
            username: Some("user@test.com".to_string()),
            token_type: Some("Bearer".to_string()),
            exp: Some(1_234_567_890),
            iat: Some(1_234_567_800),
            sub: Some("user-id".to_string()),
            aud: Some("audience".to_string()),
            iss: Some("issuer".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains(r#""active":true"#));
    }

    #[test]
    fn test_introspection_response_inactive() {
        let response = IntrospectionResponse {
            active: false,
            scope: None,
            client_id: None,
            username: None,
            token_type: None,
            exp: None,
            iat: None,
            sub: None,
            aud: None,
            iss: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains(r#""active":false"#));
        assert!(!json.contains("scope"));
    }
}
