use super::*;

impl AuthProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(Vec::new())),
            fallback_enabled: false,
        }
    }

    pub fn with_fallback(mut self, enabled: bool) -> Self {
        self.fallback_enabled = enabled;
        self
    }

    pub async fn register(&self, provider: Arc<dyn AuthProvider>) {
        let mut providers = self.providers.write().await;
        providers.push(provider);
        providers.sort_by_key(|p| p.priority());
        info!(
            "Registered auth provider: {} (priority: {})",
            providers.last().map(|p| p.name()).unwrap_or("unknown"),
            providers.last().map(|p| p.priority()).unwrap_or(0)
        );
    }

    pub async fn authenticate_token(&self, token: &str) -> Result<AuthenticatedUser, AuthError> {
        let providers = self.providers.read().await;

        for provider in providers.iter() {
            if !provider.is_enabled() {
                continue;
            }

            if !provider.supports_token_type(token) {
                continue;
            }

            match provider.authenticate(token).await {
                Ok(user) => {
                    debug!("Token authenticated via provider: {}", provider.name());
                    return Ok(user);
                }
                Err(e) => {
                    debug!("Provider {} failed: {:?}", provider.name(), e);
                    continue;
                }
            }
        }

        if self.fallback_enabled {
            warn!("All providers failed, using anonymous fallback");
            return Ok(AuthenticatedUser::anonymous());
        }

        Err(AuthError::InvalidToken)
    }

    pub async fn authenticate_api_key(
        &self,
        api_key: &str,
    ) -> Result<AuthenticatedUser, AuthError> {
        let providers = self.providers.read().await;

        for provider in providers.iter() {
            if !provider.is_enabled() {
                continue;
            }

            match provider.authenticate_api_key(api_key).await {
                Ok(user) => {
                    debug!("API key authenticated via provider: {}", provider.name());
                    return Ok(user);
                }
                Err(AuthError::InvalidApiKey) => continue,
                Err(e) => {
                    debug!("Provider {} API key auth failed: {:?}", provider.name(), e);
                    continue;
                }
            }
        }

        if self.fallback_enabled {
            warn!("All providers failed for API key, using anonymous fallback");
            return Ok(AuthenticatedUser::anonymous());
        }

        Err(AuthError::InvalidApiKey)
    }

    pub async fn provider_count(&self) -> usize {
        self.providers.read().await.len()
    }

    pub async fn list_providers(&self) -> Vec<String> {
        self.providers
            .read()
            .await
            .iter()
            .map(|p| {
                format!(
                    "{} (priority: {}, enabled: {})",
                    p.name(),
                    p.priority(),
                    p.is_enabled()
                )
            })
            .collect()
    }
}
impl Default for AuthProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
