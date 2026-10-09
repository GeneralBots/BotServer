use super::*;

impl LocalJwtAuthProvider {
    pub fn new(jwt_manager: Arc<JwtManager>) -> Self {
        Self {
            jwt_manager,
            enabled: true,
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub(super) fn claims_to_user(&self, claims: &Claims) -> Result<AuthenticatedUser, AuthError> {
        let user_id = claims.user_id().map_err(|_| AuthError::InvalidToken)?;

        let username = claims
            .username
            .clone()
            .unwrap_or_else(|| format!("user-{}", user_id));

        let roles: Vec<Role> = claims
            .roles
            .as_ref()
            .map(|r| r.iter().filter_map(|s| Role::from_str(s).ok()).collect())
            .unwrap_or_else(|| vec![Role::User]);

        let mut user = AuthenticatedUser::new(user_id, username).with_roles(roles);

        if let Some(ref email) = claims.email {
            user = user.with_email(email);
        }

        if let Some(ref session_id) = claims.session_id {
            user = user.with_session(session_id);
        }

        if let Some(ref org_id) = claims.organization_id {
            if let Ok(org_uuid) = Uuid::parse_str(org_id) {
                user = user.with_organization(org_uuid);
            }
        }

        Ok(user)
    }
}
#[async_trait]
impl AuthProvider for LocalJwtAuthProvider {
    fn name(&self) -> &str {
        "local-jwt"
    }

    fn priority(&self) -> i32 {
        100
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    async fn authenticate(&self, token: &str) -> Result<AuthenticatedUser, AuthError> {
        let claims = self.jwt_manager.validate_access_token(token).map_err(|e| {
            debug!("JWT validation failed: {e}");
            AuthError::InvalidToken
        })?;

        self.claims_to_user(&claims)
    }

    async fn authenticate_api_key(&self, _api_key: &str) -> Result<AuthenticatedUser, AuthError> {
        Err(AuthError::InvalidApiKey)
    }

    fn supports_token_type(&self, token: &str) -> bool {
        let parts: Vec<&str> = token.split('.').collect();
        parts.len() == 3
    }
}
