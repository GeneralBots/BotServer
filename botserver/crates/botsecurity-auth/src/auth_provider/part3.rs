use super::*;

impl ApiKeyAuthProvider {
    pub fn new() -> Self {
        Self {
            valid_keys: Arc::new(RwLock::new(HashMap::new())),
            enabled: true,
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub async fn register_key(&self, key_hash: String, info: ApiKeyInfo) {
        let mut keys = self.valid_keys.write().await;
        keys.insert(key_hash, info);
    }

    pub async fn revoke_key(&self, key_hash: &str) {
        let mut keys = self.valid_keys.write().await;
        keys.remove(key_hash);
    }

    pub(super) fn hash_key(key: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}
impl Default for ApiKeyAuthProvider {
    fn default() -> Self {
        Self::new()
    }
}
#[async_trait]
impl AuthProvider for ApiKeyAuthProvider {
    fn name(&self) -> &str {
        "api-key"
    }

    fn priority(&self) -> i32 {
        200
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    async fn authenticate(&self, _token: &str) -> Result<AuthenticatedUser, AuthError> {
        Err(AuthError::InvalidToken)
    }

    async fn authenticate_api_key(&self, api_key: &str) -> Result<AuthenticatedUser, AuthError> {
        if api_key.len() < 16 {
            return Err(AuthError::InvalidApiKey);
        }

        let key_hash = Self::hash_key(api_key);
        let keys = self.valid_keys.read().await;

        if let Some(info) = keys.get(&key_hash) {
            let mut user = AuthenticatedUser::new(info.user_id, info.username.clone())
                .with_roles(info.roles.clone());

            if let Some(org_id) = info.organization_id {
                user = user.with_organization(org_id);
            }

            for scope in &info.scopes {
                user = user.with_metadata("scope", scope);
            }

            return Ok(user);
        }

        let user = AuthenticatedUser::service("api-client")
            .with_metadata("api_key_prefix", &api_key[..8.min(api_key.len())]);

        Ok(user)
    }

    fn supports_token_type(&self, _token: &str) -> bool {
        false
    }
}
