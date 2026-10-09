use super::*;

impl VertexTokenManager {
    pub fn new(credentials_path: &str) -> Self {
        Self {
            credentials_path: credentials_path.to_string(),
            cached_token: Arc::new(RwLock::new(None)),
        }
    }

    /// Get a valid access token, refreshing if necessary
    pub async fn get_access_token(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Check cached token
        {
            let cache = self.cached_token.read().await;
            if let Some(ref token) = *cache {
                // Return cached token if it expires in more than 60 seconds
                if token.expires_at > std::time::Instant::now() + std::time::Duration::from_secs(60) {
                    trace!("Using cached Vertex AI access token");
                    return Ok(token.access_token.clone());
                }
            }
        }

        // Generate new token
        info!("Generating new Vertex AI access token from service account");
        let token = self.generate_token().await?;

        // Cache it
        {
            let mut cache = self.cached_token.write().await;
            *cache = Some(token.clone());
        }

        Ok(token.access_token)
    }

    async fn generate_token(&self) -> Result<CachedToken, Box<dyn std::error::Error + Send + Sync>> {
        let content = if self.credentials_path.trim().starts_with('{') {
            self.credentials_path.clone()
        } else {
            // Expand ~ to home directory
            let expanded_path = if self.credentials_path.starts_with('~') {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
                self.credentials_path.replacen('~', &home, 1)
            } else {
                self.credentials_path.clone()
            };

            tokio::fs::read_to_string(&expanded_path)
                .await
                .map_err(|e| format!("Failed to read service account key from '{}': {}", expanded_path, e))?
        };

        let sa_key: ServiceAccountKey = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse service account JSON: {}", e))?;

        let token_uri = sa_key.token_uri.as_deref().unwrap_or(GOOGLE_TOKEN_URL);

        // Create JWT assertion
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("System time error: {}", e))?
            .as_secs();

        let jwt_claims = JwtClaims {
            iss: sa_key.client_email.clone(),
            scope: GOOGLE_SCOPE.to_string(),
            aud: token_uri.to_string(),
            iat: now,
            exp: now + 3600,
        };

        let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
        let encoding_key = jsonwebtoken::EncodingKey::from_rsa_pem(sa_key.private_key.as_bytes())
            .map_err(|e| format!("Failed to parse RSA private key: {}", e))?;

        let jwt = jsonwebtoken::encode(&header, &jwt_claims, &encoding_key)
            .map_err(|e| format!("Failed to encode JWT: {}", e))?;

        // Exchange JWT for access token
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        let response = client
            .post(token_uri)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", &jwt),
            ])
            .send()
            .await
            .map_err(|e| format!("Failed to request token: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            log::error!("Google OAuth2 token request failed: {} - {}", status, body);
            return Err(format!("Token request failed with status {}: {}", status, body).into());
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse token response: {}", e))?;

        info!("Successfully obtained Vertex AI access token (expires in {}s)", token_response.expires_in);

        Ok(CachedToken {
            access_token: token_response.access_token,
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(token_response.expires_in),
        })
    }
}
