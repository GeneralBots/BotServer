use super::*;

impl ZitadelAuthProviderAdapter {
    pub fn new(provider: Arc<ZitadelAuthProvider>) -> Self {
        Self {
            provider,
            enabled: true,
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}
#[async_trait]
impl AuthProvider for ZitadelAuthProviderAdapter {
    fn name(&self) -> &str {
        "zitadel"
    }

    fn priority(&self) -> i32 {
        50
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    async fn authenticate(&self, token: &str) -> Result<AuthenticatedUser, AuthError> {
        self.provider.authenticate_token(token).await
    }

    async fn authenticate_api_key(&self, api_key: &str) -> Result<AuthenticatedUser, AuthError> {
        self.provider.authenticate_api_key(api_key).await
    }

    fn supports_token_type(&self, token: &str) -> bool {
        let parts: Vec<&str> = token.split('.').collect();
        parts.len() == 3
    }
}
