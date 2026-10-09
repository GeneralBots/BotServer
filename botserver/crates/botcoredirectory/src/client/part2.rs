use super::*;

impl ZitadelClient {
    pub(super) fn sanitize_api_base(raw: &str, allow_insecure_http: bool) -> Result<String> {
        let parsed = url::Url::parse(raw).map_err(|e| anyhow!("invalid Zitadel api_url: {e}"))?;
        let host = parsed.host_str().ok_or_else(|| anyhow!("Zitadel api_url requires a host"))?.to_string();
        // Bearer tokens and user data travel to this host: require https for
        // public destinations. Plain http is allowed when traffic provably
        // stays inside a trusted perimeter (loopback/private networks) or when
        // the operator explicitly opted in via `allow_insecure_http`.
        anyhow::ensure!(
            parsed.scheme() == "https"
                || (parsed.scheme() == "http" && (allow_insecure_http || is_private_host(&host))),
            "Zitadel api_url must be https (or http on loopback/private networks; set allow_insecure_http to override)"
        );
        let mut base = format!("{}://{}", parsed.scheme(), host);
        if let Some(port) = parsed.port() {
            base.push_str(&format!(":{port}"));
        }
        Ok(base)
    }

    pub fn new(config: ZitadelConfig) -> Result<Self> {
        let api_base = Self::sanitize_api_base(&config.api_url, config.allow_insecure_http)?;
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            config,
            api_base,
            http_client,
            access_token: Arc::new(RwLock::new(None)),
            token_expires_at: Arc::new(RwLock::new(None)),
            pat_token: None,
            password_credentials: None,
        })
    }

    /// Create a client that uses password grant OAuth flow
    /// This is used for initial bootstrap with Zitadel's default admin user
    pub fn with_password_grant(
        config: ZitadelConfig,
        username: String,
        password: String,
    ) -> Result<Self> {
        let api_base = Self::sanitize_api_base(&config.api_url, config.allow_insecure_http)?;
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            config,
            api_base,
            http_client,
            access_token: Arc::new(RwLock::new(None)),
            token_expires_at: Arc::new(RwLock::new(None)),
            pat_token: None,
            password_credentials: Some((username, password)),
        })
    }

    pub fn with_pat_token(config: ZitadelConfig, pat_token: String) -> Result<Self> {
        let api_base = Self::sanitize_api_base(&config.api_url, config.allow_insecure_http)?;
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            config,
            api_base,
            http_client,
            access_token: Arc::new(RwLock::new(None)),
            token_expires_at: Arc::new(RwLock::new(None)),
            pat_token: Some(pat_token),
            password_credentials: None,
        })
    }

    pub fn set_pat_token(&mut self, token: String) {
        self.pat_token = Some(token);
    }

    pub fn api_url(&self) -> String {
        self.api_base.clone()
    }

    pub fn client_id(&self) -> String {
        self.config.client_id.clone()
    }

    pub fn client_secret(&self) -> String {
        self.config.client_secret.clone()
    }

    pub async fn http_get(&self, url: String) -> Result<reqwest::RequestBuilder> {
        let token = self.get_access_token().await
            .map_err(|e| anyhow!("Token acquisition failed for GET {}: {}", url, e))?;
        Ok(self.http_client.get(url).bearer_auth(token))
    }

    pub async fn http_post(&self, url: String) -> Result<reqwest::RequestBuilder> {
        let token = self.get_access_token().await
            .map_err(|e| anyhow!("Token acquisition failed for POST {}: {}", url, e))?;
        Ok(self.http_client.post(url).bearer_auth(token))
    }

    pub async fn http_put(&self, url: String) -> Result<reqwest::RequestBuilder> {
        let token = self.get_access_token().await
            .map_err(|e| anyhow!("Token acquisition failed for PUT {}: {}", url, e))?;
        Ok(self.http_client.put(url).bearer_auth(token))
    }

    pub async fn http_patch(&self, url: String) -> Result<reqwest::RequestBuilder> {
        let token = self.get_access_token().await
            .map_err(|e| anyhow!("Token acquisition failed for PATCH {}: {}", url, e))?;
        Ok(self.http_client.patch(url).bearer_auth(token))
    }

    pub async fn http_delete(&self, url: String) -> Result<reqwest::RequestBuilder> {
        let token = self.get_access_token().await
            .map_err(|e| anyhow!("Token acquisition failed for DELETE {}: {}", url, e))?;
        Ok(self.http_client.delete(url).bearer_auth(token))
    }

    pub async fn get_access_token(&self) -> Result<String> {
        if let Some(ref pat) = self.pat_token {
            return Ok(pat.clone());
        }

        // Check cached token with expiry
        {
            let token = self.access_token.read().await;
            let expires = self.token_expires_at.read().await;
            if let Some(t) = token.as_ref() {
                let still_valid = expires.map(|e| Instant::now() < e).unwrap_or(true);
                if still_valid {
                    return Ok(t.clone());
                }
                log::info!("Cached access token expired, refreshing...");
            }
        }

        let token_url = format!("{}/oauth/v2/token", self.api_base);
        log::info!("Requesting access token from: {}", token_url);

        // Build params dynamically based on auth method
        let mut params: Vec<(&str, String)> = vec![
            ("client_id", self.config.client_id.clone()),
            ("client_secret", self.config.client_secret.clone()),
        ];

        if let Some((username, password)) = &self.password_credentials {
            // Use password grant flow
            params.push(("grant_type", "password".to_string()));
            params.push(("username", username.clone()));
            params.push(("password", password.clone()));
            params.push(("scope", "openid profile email urn:zitadel:iam:org:project:id:zitadel:aud".to_string()));
        } else {
            // Use client credentials flow
            params.push(("grant_type", "client_credentials".to_string()));
            params.push(("scope", "openid profile email urn:zitadel:iam:org:project:id:zitadel:aud".to_string()));
        }

        let response = self
            .http_client
            .post(&token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to get access token: {}", e))?;

        let token_data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse token response: {}", e))?;

        let access_token = token_data
            .get("access_token")
            .and_then(|t| t.as_str())
            .ok_or_else(|| anyhow!("No access token in response"))?
            .to_string();

        // Calculate expiry with 60s safety margin
        let expires_in = token_data
            .get("expires_in")
            .and_then(|t| t.as_i64())
            .unwrap_or(3600);
        let expires_at = Instant::now()
            + std::time::Duration::from_secs(expires_in.max(60) as u64 - 60);

        {
            let mut token = self.access_token.write().await;
            *token = Some(access_token.clone());
            let mut expires = self.token_expires_at.write().await;
            *expires = Some(expires_at);
        }

        Ok(access_token)
    }

    pub async fn create_user(
        &self,
        email: &str,
        first_name: &str,
        last_name: &str,
        username: Option<&str>,
    ) -> Result<String> {
        self.create_user_with_password(email, first_name, last_name, username, None).await
    }
}
