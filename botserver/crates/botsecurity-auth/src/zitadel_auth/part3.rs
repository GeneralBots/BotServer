use super::*;

impl ZitadelAuthProvider {
    pub fn new(config: ZitadelAuthConfig) -> Result<Self> {
        let http_client = create_tls_client(Some(30));

        Ok(Self {
            config,
            http_client,
            user_cache: Arc::new(RwLock::new(HashMap::new())),
            service_token: Arc::new(RwLock::new(None)),
        })
    }

    pub async fn authenticate_request(
        &self,
        request: &Request<Body>,
        auth_config: &AuthConfig,
    ) -> Result<AuthenticatedUser, AuthError> {
        if let Some(token) = self.extract_bearer_token(request, auth_config) {
            return self.authenticate_token(&token).await;
        }

        if let Some(api_key) = self.extract_api_key(request, auth_config) {
            return self.authenticate_api_key(&api_key).await;
        }

        Err(AuthError::MissingToken)
    }

    pub(super) fn extract_bearer_token(&self, request: &Request<Body>, config: &AuthConfig) -> Option<String> {
        request
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|auth| auth.strip_prefix(&config.bearer_prefix))
            .map(|s| s.to_string())
    }

    pub(super) fn extract_api_key(&self, request: &Request<Body>, config: &AuthConfig) -> Option<String> {
        request
            .headers()
            .get(&config.api_key_header)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    }

    pub async fn authenticate_token(&self, token: &str) -> Result<AuthenticatedUser, AuthError> {
        if let Some(cached) = self.get_cached_user(token).await {
            return Ok(cached);
        }

        let user = if self.config.introspect_tokens {
            self.introspect_and_get_user(token).await?
        } else {
            self.decode_jwt_user(token)?
        };

        self.cache_user(token, &user).await;

        Ok(user)
    }

    pub async fn authenticate_api_key(&self, api_key: &str) -> Result<AuthenticatedUser, AuthError> {
        if api_key.len() < 16 {
            return Err(AuthError::InvalidApiKey);
        }

        if let Some(cached) = self.get_cached_user(api_key).await {
            return Ok(cached);
        }

        let user = self.validate_api_key_with_zitadel(api_key).await?;

        self.cache_user(api_key, &user).await;

        Ok(user)
    }

    async fn introspect_and_get_user(&self, token: &str) -> Result<AuthenticatedUser, AuthError> {
        let introspect_url = format!("{}/oauth/v2/introspect", self.config.sanitized_api_base().unwrap_or_default());

        let params = [
            ("token", token),
            ("client_id", &self.config.client_id),
            ("client_secret", &self.config.client_secret),
        ];

        let response = self
            .http_client
            .post(&introspect_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                log::error!("Token introspection request failed: {}", e);
                AuthError::InternalError("Authentication service unavailable".to_string())
            })?;

        if !response.status().is_success() {
            warn!("Token introspection failed with status: {}", response.status());
            return Err(AuthError::InvalidToken);
        }

        let introspection: serde_json::Value = response.json().await.map_err(|e| {
            log::error!("Failed to parse introspection response: {}", e);
            AuthError::InternalError("Invalid authentication response".to_string())
        })?;

        let active = introspection
            .get("active")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !active {
            return Err(AuthError::ExpiredToken);
        }

        let user_id = introspection
            .get("sub")
            .and_then(|v| v.as_str())
            .ok_or(AuthError::InvalidToken)?;

        let username = introspection
            .get("username")
            .or_else(|| introspection.get("preferred_username"))
            .and_then(|v| v.as_str())
            .unwrap_or(user_id);

        let email = introspection
            .get("email")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let roles: Vec<String> = introspection
            .get("roles")
            .or_else(|| {
                introspection
                    .get(format!("urn:zitadel:iam:org:project:{}:roles", self.config.project_id))
            })
            .and_then(|v| v.as_object())
            .map(|obj| obj.keys().cloned().collect())
            .unwrap_or_default();

        let zitadel_user = ZitadelUser {
            id: user_id.to_string(),
            username: username.to_string(),
            email,
            email_verified: introspection
                .get("email_verified")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            first_name: introspection
                .get("given_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            last_name: introspection
                .get("family_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            display_name: introspection
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            roles,
            organization_id: introspection
                .get("urn:zitadel:iam:user:resourceowner:id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            metadata: HashMap::new(),
        };

        zitadel_user.to_authenticated_user()
    }

    pub(super) fn decode_jwt_user(&self, token: &str) -> Result<AuthenticatedUser, AuthError> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(AuthError::InvalidToken);
        }

        let payload = parts[1];
        let decoded = base64_url_decode(payload).map_err(|_| AuthError::InvalidToken)?;

        let claims: serde_json::Value =
            serde_json::from_slice(&decoded).map_err(|_| AuthError::InvalidToken)?;

        let user_id = claims
            .get("sub")
            .and_then(|v| v.as_str())
            .ok_or(AuthError::InvalidToken)?;

        let username = claims
            .get("preferred_username")
            .or_else(|| claims.get("username"))
            .and_then(|v| v.as_str())
            .unwrap_or(user_id);

        let exp = claims
            .get("exp")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        if exp > 0 && exp < chrono::Utc::now().timestamp() {
            return Err(AuthError::ExpiredToken);
        }

        let roles: Vec<String> = claims
            .get("roles")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        let zitadel_user = ZitadelUser {
            id: user_id.to_string(),
            username: username.to_string(),
            email: claims
                .get("email")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            email_verified: claims
                .get("email_verified")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            first_name: claims
                .get("given_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            last_name: claims
                .get("family_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            display_name: claims
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            roles,
            organization_id: None,
            metadata: HashMap::new(),
        };

        zitadel_user.to_authenticated_user()
    }

    async fn validate_api_key_with_zitadel(
        &self,
        api_key: &str,
    ) -> Result<AuthenticatedUser, AuthError> {
        let service_token = self.get_service_token().await?;

        let url = format!("{}/v2/users/_search", self.config.sanitized_api_base().unwrap_or_default());

        let body = serde_json::json!({
            "queries": [{
                "typeQuery": {
                    "type": "TYPE_MACHINE"
                }
            }],
            "limit": 1
        });

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&service_token)
            .json(&body)
            .header("x-zitadel-api-key", api_key)
            .send()
            .await
            .map_err(|e| {
                log::error!("API key validation request failed: {}", e);
                AuthError::InternalError("Authentication service unavailable".to_string())
            })?;

        if !response.status().is_success() {
            return Err(AuthError::InvalidApiKey);
        }

        Ok(AuthenticatedUser::service("api-key-user")
            .with_metadata("api_key_prefix", botlib::text::truncate_at_char_boundary(api_key, 8)))
    }
}
