pub(crate) use crate::auth::{AuthConfig, AuthError, AuthenticatedUser, Role};
pub(crate) use crate::jwt::{Claims, JwtManager};
pub(crate) use crate::zitadel_auth::{ZitadelAuthConfig, ZitadelAuthProvider};
pub(crate) use anyhow::Result;
pub(crate) use async_trait::async_trait;
pub(crate) use std::collections::HashMap;
pub(crate) use std::str::FromStr;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use tracing::{debug, info, warn};
pub(crate) use uuid::Uuid;

#[async_trait]
pub trait AuthProvider: Send + Sync {
    fn name(&self) -> &str;
    fn priority(&self) -> i32;
    fn is_enabled(&self) -> bool;
    async fn authenticate(&self, token: &str) -> Result<AuthenticatedUser, AuthError>;
    async fn authenticate_api_key(&self, api_key: &str) -> Result<AuthenticatedUser, AuthError>;
    fn supports_token_type(&self, token: &str) -> bool;
}

pub struct LocalJwtAuthProvider {
    jwt_manager: Arc<JwtManager>,
    enabled: bool,
}

pub struct ZitadelAuthProviderAdapter {
    provider: Arc<ZitadelAuthProvider>,
    enabled: bool,
}

pub struct ApiKeyAuthProvider {
    valid_keys: Arc<RwLock<HashMap<String, ApiKeyInfo>>>,
    enabled: bool,
}

#[derive(Clone)]
pub struct ApiKeyInfo {
    pub user_id: Uuid,
    pub username: String,
    pub roles: Vec<Role>,
    pub organization_id: Option<Uuid>,
    pub scopes: Vec<String>,
}

pub struct AuthProviderRegistry {
    providers: Arc<RwLock<Vec<Arc<dyn AuthProvider>>>>,
    fallback_enabled: bool,
}

pub struct AuthProviderBuilder {
    jwt_manager: Option<Arc<JwtManager>>,
    zitadel_provider: Option<Arc<ZitadelAuthProvider>>,
    zitadel_config: Option<ZitadelAuthConfig>,
    auth_config: Option<Arc<AuthConfig>>,
    api_key_provider: Option<Arc<ApiKeyAuthProvider>>,
    extra_providers: Vec<Arc<dyn AuthProvider>>,
    fallback_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_jwt_manager() -> Arc<JwtManager> {
        let config = crate::jwt::JwtConfig::default();
        let key = crate::jwt::JwtKey::from_secret("test-secret-key-for-testing-only");
        Arc::new(JwtManager::new(config, key).expect("Failed to create JwtManager"))
    }

    #[tokio::test]
    async fn test_registry_creation() {
        let registry = AuthProviderRegistry::new();
        assert_eq!(registry.provider_count().await, 0);
    }

    #[tokio::test]
    async fn test_register_provider() {
        let registry = AuthProviderRegistry::new();
        let jwt_manager = create_test_jwt_manager();
        let provider = Arc::new(LocalJwtAuthProvider::new(jwt_manager));

        registry.register(provider).await;

        assert_eq!(registry.provider_count().await, 1);
    }

    #[tokio::test]
    async fn test_jwt_provider_validates_token() {
        let jwt_manager = create_test_jwt_manager();
        let provider = LocalJwtAuthProvider::new(Arc::clone(&jwt_manager));

        let token_pair = jwt_manager
            .generate_token_pair(Uuid::new_v4())
            .expect("Failed to generate token");

        let result = provider.authenticate(&token_pair.access_token).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_jwt_provider_rejects_invalid_token() {
        let jwt_manager = create_test_jwt_manager();
        let provider = LocalJwtAuthProvider::new(jwt_manager);

        let result = provider.authenticate("invalid.token.here").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_api_key_provider() {
        let provider = ApiKeyAuthProvider::new();

        let info = ApiKeyInfo {
            user_id: Uuid::new_v4(),
            username: "test-user".to_string(),
            roles: vec![Role::User],
            organization_id: None,
            scopes: vec!["read".to_string()],
        };

        let key = "test-api-key-12345678";
        let key_hash = ApiKeyAuthProvider::hash_key(key);
        provider.register_key(key_hash, info).await;

        let result = provider.authenticate_api_key(key).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_registry_with_fallback() {
        let registry = AuthProviderRegistry::new().with_fallback(true);

        let result = registry.authenticate_token("invalid-token").await;
        assert!(result.is_ok());

        let user = result.expect("Expected anonymous user");
        assert!(!user.is_authenticated());
    }

    #[tokio::test]
    async fn test_registry_without_fallback() {
        let registry = AuthProviderRegistry::new().with_fallback(false);

        let result = registry.authenticate_token("invalid-token").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_builder_pattern() {
        let jwt_manager = create_test_jwt_manager();

        let registry = AuthProviderBuilder::new()
            .with_jwt_manager(jwt_manager)
            .with_api_key_provider(Arc::new(ApiKeyAuthProvider::new()))
            .with_fallback(false)
            .build()
            .await;

        assert_eq!(registry.provider_count().await, 2);
    }

    #[tokio::test]
    async fn test_list_providers() {
        let jwt_manager = create_test_jwt_manager();
        let registry = AuthProviderRegistry::new();

        let provider = Arc::new(LocalJwtAuthProvider::new(jwt_manager));
        registry.register(provider).await;

        let providers = registry.list_providers().await;
        assert_eq!(providers.len(), 1);
        assert!(providers[0].contains("local-jwt"));
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

pub use part5::*;
