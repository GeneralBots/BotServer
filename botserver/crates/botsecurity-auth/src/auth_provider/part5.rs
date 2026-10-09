use super::*;

impl AuthProviderBuilder {
    pub fn new() -> Self {
        Self {
            jwt_manager: None,
            zitadel_provider: None,
            zitadel_config: None,
            auth_config: None,
            api_key_provider: None,
            extra_providers: Vec::new(),
            fallback_enabled: false,
        }
    }

    pub fn with_jwt_manager(mut self, manager: Arc<JwtManager>) -> Self {
        self.jwt_manager = Some(manager);
        self
    }

    pub fn with_zitadel(
        mut self,
        provider: Arc<ZitadelAuthProvider>,
        config: ZitadelAuthConfig,
    ) -> Self {
        self.zitadel_provider = Some(provider);
        self.zitadel_config = Some(config);
        self
    }

    pub fn with_auth_config(mut self, config: Arc<AuthConfig>) -> Self {
        self.auth_config = Some(config);
        self
    }

    pub fn with_api_key_provider(mut self, provider: Arc<ApiKeyAuthProvider>) -> Self {
        self.api_key_provider = Some(provider);
        self
    }

    pub fn with_provider(mut self, provider: Arc<dyn AuthProvider>) -> Self {
        self.extra_providers.push(provider);
        self
    }

    pub fn with_fallback(mut self, enabled: bool) -> Self {
        self.fallback_enabled = enabled;
        self
    }

    pub async fn build(self) -> AuthProviderRegistry {
        let registry = AuthProviderRegistry::new().with_fallback(self.fallback_enabled);

        if let Some(jwt_manager) = self.jwt_manager {
            let provider = Arc::new(LocalJwtAuthProvider::new(jwt_manager));
            registry.register(provider).await;
        }

        if let (Some(zitadel), Some(_config)) = (self.zitadel_provider, self.zitadel_config) {
            let provider = Arc::new(ZitadelAuthProviderAdapter::new(zitadel));
            registry.register(provider).await;
        }

        if let Some(api_key_provider) = self.api_key_provider {
            registry.register(api_key_provider).await;
        }

        for provider in self.extra_providers {
            registry.register(provider).await;
        }

        registry
    }
}
impl Default for AuthProviderBuilder {
    fn default() -> Self {
        Self::new()
    }
}
pub async fn create_default_registry(
    jwt_secret: &str,
    zitadel_config: Option<ZitadelAuthConfig>,
) -> Result<AuthProviderRegistry> {
    let jwt_config = crate::jwt::JwtConfig::default();
    let jwt_key = crate::jwt::JwtKey::from_secret(&jwt_secret);
    let jwt_manager = Arc::new(crate::jwt::JwtManager::new(jwt_config, jwt_key)?);

    let mut builder = AuthProviderBuilder::new()
        .with_jwt_manager(jwt_manager)
        .with_api_key_provider(Arc::new(ApiKeyAuthProvider::new()))
        .with_fallback(false);

    if let Some(config) = zitadel_config {
        if config.is_configured() {
            match ZitadelAuthProvider::new(config.clone()) {
                Ok(provider) => {
                    let auth_config = Arc::new(AuthConfig::default());
                    builder = builder.with_zitadel(Arc::new(provider), config);
                    builder = builder.with_auth_config(auth_config);
                    info!("Zitadel authentication provider configured");
                }
                Err(e) => {
                    log::error!("Failed to create Zitadel provider: {e}");
                }
            }
        }
    }

    Ok(builder.build().await)
}
