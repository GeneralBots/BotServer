use super::*;

impl SocialOAuthService {
    pub fn new() -> Self {
        Self {
            credentials: Arc::new(RwLock::new(HashMap::new())),
            tokens: Arc::new(RwLock::new(HashMap::new())),
            states: Arc::new(RwLock::new(HashMap::new())),
            accounts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_credentials(&self, credentials: OAuthCredentials) {
        let mut creds = self.credentials.write().await;
        creds.insert(credentials.platform, credentials);
    }

    pub async fn get_credentials(&self, platform: SocialPlatform) -> Option<OAuthCredentials> {
        let creds = self.credentials.read().await;
        creds.get(&platform).cloned()
    }

    pub async fn generate_authorization_url(
        &self,
        platform: SocialPlatform,
        organization_id: Uuid,
        user_id: Uuid,
        redirect_after: Option<String>,
        additional_scopes: Option<Vec<String>>,
    ) -> Result<AuthorizationUrl, OAuthServiceError> {
        let credentials = self
            .get_credentials(platform)
            .await
            .ok_or(OAuthServiceError::CredentialsNotFound)?;

        let auth_url = platform
            .authorization_url()
            .ok_or(OAuthServiceError::PlatformNotSupported)?;

        let state = OAuthState::new(platform, organization_id, user_id, redirect_after);
        let state_encoded = state.encode();

        {
            let mut states = self.states.write().await;
            states.insert(state_encoded.clone(), state);
        }

        let mut scopes: Vec<String> = platform
            .default_scopes()
            .into_iter()
            .map(String::from)
            .collect();

        if let Some(additional) = additional_scopes {
            scopes.extend(additional);
        }

        let scope_string = scopes.join(" ");

        let url = format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
            auth_url,
            urlencoding::encode(&credentials.client_id),
            urlencoding::encode(&credentials.redirect_uri),
            urlencoding::encode(&scope_string),
            urlencoding::encode(&state_encoded)
        );

        Ok(AuthorizationUrl {
            url,
            state: state_encoded,
        })
    }

    pub async fn exchange_code(
        &self,
        request: TokenExchangeRequest,
    ) -> Result<OAuthToken, OAuthServiceError> {
        let state = {
            let mut states = self.states.write().await;
            states
                .remove(&request.state)
                .ok_or(OAuthServiceError::InvalidState)?
        };

        if state.is_expired() {
            return Err(OAuthServiceError::StateExpired);
        }

        let credentials = self
            .get_credentials(state.platform)
            .await
            .ok_or(OAuthServiceError::CredentialsNotFound)?;

        let token_url = state
            .platform
            .token_url()
            .ok_or(OAuthServiceError::PlatformNotSupported)?;

        let client = reqwest::Client::new();
        let response = client
            .post(token_url)
            .form(&[
                ("client_id", credentials.client_id.as_str()),
                ("client_secret", credentials.client_secret.as_str()),
                ("code", request.code.as_str()),
                ("redirect_uri", credentials.redirect_uri.as_str()),
                ("grant_type", "authorization_code"),
            ])
            .send()
            .await
            .map_err(|e| OAuthServiceError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OAuthServiceError::TokenExchangeFailed(error_text));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| OAuthServiceError::ParseError(e.to_string()))?;

        let now = Utc::now();
        let expires_at = token_response
            .expires_in
            .map(|secs| now + chrono::Duration::seconds(secs));

        let scopes = token_response
            .scope
            .map(|s| s.split(' ').map(String::from).collect())
            .unwrap_or_default();

        let token = OAuthToken {
            id: Uuid::new_v4(),
            organization_id: state.organization_id,
            platform: state.platform,
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            token_type: token_response.token_type,
            expires_at,
            scopes,
            account_id: None,
            account_name: None,
            created_at: now,
            updated_at: now,
        };

        {
            let mut tokens = self.tokens.write().await;
            tokens.insert(token.id, token.clone());
        }

        Ok(token)
    }

    pub async fn refresh_token(&self, token_id: Uuid) -> Result<OAuthToken, OAuthServiceError> {
        let existing_token = {
            let tokens = self.tokens.read().await;
            tokens
                .get(&token_id)
                .cloned()
                .ok_or(OAuthServiceError::TokenNotFound)?
        };

        let refresh_token = existing_token
            .refresh_token
            .as_ref()
            .ok_or(OAuthServiceError::NoRefreshToken)?;

        let credentials = self
            .get_credentials(existing_token.platform)
            .await
            .ok_or(OAuthServiceError::CredentialsNotFound)?;

        let token_url = existing_token
            .platform
            .token_url()
            .ok_or(OAuthServiceError::PlatformNotSupported)?;

        let client = reqwest::Client::new();
        let response = client
            .post(token_url)
            .form(&[
                ("client_id", credentials.client_id.as_str()),
                ("client_secret", credentials.client_secret.as_str()),
                ("refresh_token", refresh_token.as_str()),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await
            .map_err(|e| OAuthServiceError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OAuthServiceError::TokenRefreshFailed(error_text));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| OAuthServiceError::ParseError(e.to_string()))?;

        let now = Utc::now();
        let expires_at = token_response
            .expires_in
            .map(|secs| now + chrono::Duration::seconds(secs));

        let updated_token = OAuthToken {
            id: existing_token.id,
            organization_id: existing_token.organization_id,
            platform: existing_token.platform,
            access_token: token_response.access_token,
            refresh_token: token_response
                .refresh_token
                .or(existing_token.refresh_token),
            token_type: token_response.token_type,
            expires_at,
            scopes: existing_token.scopes,
            account_id: existing_token.account_id,
            account_name: existing_token.account_name,
            created_at: existing_token.created_at,
            updated_at: now,
        };

        {
            let mut tokens = self.tokens.write().await;
            tokens.insert(updated_token.id, updated_token.clone());
        }

        Ok(updated_token)
    }

    pub async fn revoke_token(&self, token_id: Uuid) -> Result<(), OAuthServiceError> {
        let mut tokens = self.tokens.write().await;
        tokens
            .remove(&token_id)
            .ok_or(OAuthServiceError::TokenNotFound)?;
        Ok(())
    }

    pub async fn get_token(&self, token_id: Uuid) -> Option<OAuthToken> {
        let tokens = self.tokens.read().await;
        tokens.get(&token_id).cloned()
    }

    pub async fn get_organization_tokens(&self, organization_id: Uuid) -> Vec<OAuthToken> {
        let tokens = self.tokens.read().await;
        tokens
            .values()
            .filter(|t| t.organization_id == organization_id)
            .cloned()
            .collect()
    }

    pub async fn get_platform_token(
        &self,
        organization_id: Uuid,
        platform: SocialPlatform,
    ) -> Option<OAuthToken> {
        let tokens = self.tokens.read().await;
        tokens
            .values()
            .find(|t| t.organization_id == organization_id && t.platform == platform)
            .cloned()
    }

    pub async fn get_valid_token(
        &self,
        organization_id: Uuid,
        platform: SocialPlatform,
    ) -> Result<OAuthToken, OAuthServiceError> {
        let token = self
            .get_platform_token(organization_id, platform)
            .await
            .ok_or(OAuthServiceError::TokenNotFound)?;

        if token.needs_refresh() {
            if token.refresh_token.is_some() {
                return self.refresh_token(token.id).await;
            }
            return Err(OAuthServiceError::TokenExpired);
        }

        Ok(token)
    }

    pub async fn add_connected_account(&self, account: ConnectedAccount) {
        let mut accounts = self.accounts.write().await;
        accounts.insert(account.id, account);
    }

    pub async fn get_connected_accounts(&self, organization_id: Uuid) -> Vec<ConnectedAccount> {
        let accounts = self.accounts.read().await;
        accounts
            .values()
            .filter(|a| a.organization_id == organization_id)
            .cloned()
            .collect()
    }
}
