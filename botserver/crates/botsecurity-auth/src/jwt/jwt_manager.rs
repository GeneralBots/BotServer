
use anyhow::{anyhow, Result};
use std::sync::Arc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;
use tracing::info;
use crate::blacklist::{BlacklistEntry, BlacklistStore, InMemoryBlacklistStore};
use crate::jwt::jwt_types::{Claims, JwtAlgorithm, JwtConfig, JwtKey, JwtManager, TokenPair, TokenType};

impl JwtManager {
    pub fn new(config: JwtConfig, key: JwtKey) -> Result<Self> {
        let (encoding_key, decoding_key) = match (&config.algorithm, key) {
            (JwtAlgorithm::HS256 | JwtAlgorithm::HS384 | JwtAlgorithm::HS512, JwtKey::Symmetric(secret)) => {
                (
                    EncodingKey::from_secret(&secret),
                    DecodingKey::from_secret(&secret),
                )
            }
            (JwtAlgorithm::RS256 | JwtAlgorithm::RS384 | JwtAlgorithm::RS512, JwtKey::RsaPrivate(pem)) => {
                let encoding = EncodingKey::from_rsa_pem(&pem)
                    .map_err(|e| anyhow!("Invalid RSA private key: {e}"))?;
                let decoding = DecodingKey::from_rsa_pem(&pem)
                    .map_err(|e| anyhow!("Invalid RSA key for decoding: {e}"))?;
                (encoding, decoding)
            }
            (JwtAlgorithm::ES256 | JwtAlgorithm::ES384, JwtKey::EcPrivate(pem)) => {
                let encoding = EncodingKey::from_ec_pem(&pem)
                    .map_err(|e| anyhow!("Invalid EC private key: {e}"))?;
                let decoding = DecodingKey::from_ec_pem(&pem)
                    .map_err(|e| anyhow!("Invalid EC key for decoding: {e}"))?;
                (encoding, decoding)
            }
            _ => return Err(anyhow!("Key type does not match algorithm")),
        };

        Ok(Self {
            config,
            encoding_key,
            decoding_key,
            blacklist: Arc::new(InMemoryBlacklistStore::new()),
        })
    }

    /// Replaces the default in-memory blacklist with a custom store (e.g. a
    /// Redis/Valkey-backed store so revocations survive restarts).
    pub fn with_blacklist_store(mut self, store: Arc<dyn BlacklistStore>) -> Self {
        self.blacklist = store;
        self
    }

    pub fn with_separate_keys(
        config: JwtConfig,
        signing_key: &JwtKey,
        verification_key: &JwtKey,
    ) -> Result<Self> {
        let encoding_key = match (&config.algorithm, signing_key) {
            (JwtAlgorithm::HS256 | JwtAlgorithm::HS384 | JwtAlgorithm::HS512, JwtKey::Symmetric(secret)) => {
                EncodingKey::from_secret(secret)
            }
            (JwtAlgorithm::RS256 | JwtAlgorithm::RS384 | JwtAlgorithm::RS512, JwtKey::RsaPrivate(pem)) => {
                EncodingKey::from_rsa_pem(pem)
                    .map_err(|e| anyhow!("Invalid RSA private key: {e}"))?
            }
            (JwtAlgorithm::ES256 | JwtAlgorithm::ES384, JwtKey::EcPrivate(pem)) => {
                EncodingKey::from_ec_pem(pem)
                    .map_err(|e| anyhow!("Invalid EC private key: {e}"))?
            }
            _ => return Err(anyhow!("Signing key type does not match algorithm")),
        };

        let decoding_key = match (&config.algorithm, verification_key) {
            (JwtAlgorithm::HS256 | JwtAlgorithm::HS384 | JwtAlgorithm::HS512, JwtKey::Symmetric(secret)) => {
                DecodingKey::from_secret(secret)
            }
            (JwtAlgorithm::RS256 | JwtAlgorithm::RS384 | JwtAlgorithm::RS512, JwtKey::RsaPublic(pem)) => {
                DecodingKey::from_rsa_pem(pem)
                    .map_err(|e| anyhow!("Invalid RSA public key: {e}"))?
            }
            (JwtAlgorithm::ES256 | JwtAlgorithm::ES384, JwtKey::EcPublic(pem)) => {
                DecodingKey::from_ec_pem(pem)
                    .map_err(|e| anyhow!("Invalid EC public key: {e}"))?
            }
            _ => return Err(anyhow!("Verification key type does not match algorithm")),
        };

        Ok(Self {
            config,
            encoding_key,
            decoding_key,
            blacklist: Arc::new(InMemoryBlacklistStore::new()),
        })
    }

    pub fn from_secret(secret: &str) -> Result<Self> {
        if secret.len() < 32 {
            return Err(anyhow!("JWT secret must be at least 32 characters"));
        }
        let key = JwtKey::from_secret(secret);
        Self::new(JwtConfig::default(), key)
    }

    pub fn generate_access_token(&self, claims: Claims) -> Result<String> {
        let header = Header::new(self.config.algorithm.to_jsonwebtoken());
        encode(&header, &claims, &self.encoding_key)
            .map_err(|e| anyhow!("Failed to encode access token: {e}"))
    }

    pub fn generate_refresh_token(&self, claims: Claims) -> Result<String> {
        let header = Header::new(self.config.algorithm.to_jsonwebtoken());
        encode(&header, &claims, &self.encoding_key)
            .map_err(|e| anyhow!("Failed to encode refresh token: {e}"))
    }

    pub fn generate_token_pair(&self, user_id: Uuid) -> Result<TokenPair> {
        let now = Utc::now();
        let access_expiry = now + Duration::minutes(self.config.access_token_expiry_minutes);
        let refresh_expiry = now + Duration::days(self.config.refresh_token_expiry_days);

        let access_claims = Claims::new(
            user_id,
            &self.config.issuer,
            &self.config.audience,
            TokenType::Access,
            access_expiry,
        )?;

        let refresh_claims = Claims::new(
            user_id,
            &self.config.issuer,
            &self.config.audience,
            TokenType::Refresh,
            refresh_expiry,
        )?;

        let access_token = self.generate_access_token(access_claims)?;
        let refresh_token = self.generate_refresh_token(refresh_claims)?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer".into(),
            expires_in: self.config.access_token_expiry_minutes * 60,
            refresh_expires_in: self.config.refresh_token_expiry_days * 24 * 60 * 60,
            id_token: None,
            scope: None,
        })
    }

    pub fn generate_token_pair_with_claims(
        &self,
        user_id: Uuid,
        email: Option<String>,
        username: Option<String>,
        roles: Option<Vec<String>>,
        session_id: Option<String>,
    ) -> Result<TokenPair> {
        let now = Utc::now();
        let access_expiry = now + Duration::minutes(self.config.access_token_expiry_minutes);
        let refresh_expiry = now + Duration::days(self.config.refresh_token_expiry_days);

        let mut access_claims = Claims::new(
            user_id,
            &self.config.issuer,
            &self.config.audience,
            TokenType::Access,
            access_expiry,
        )?;

        if let Some(e) = email.clone() {
            access_claims = access_claims.with_email(e);
        }
        if let Some(u) = username.clone() {
            access_claims = access_claims.with_username(u);
        }
        if let Some(r) = roles.clone() {
            access_claims = access_claims.with_roles(r);
        }
        if let Some(s) = session_id.clone() {
            access_claims = access_claims.with_session_id(s);
        }

        let mut refresh_claims = Claims::new(
            user_id,
            &self.config.issuer,
            &self.config.audience,
            TokenType::Refresh,
            refresh_expiry,
        )?;

        if let Some(e) = email {
            refresh_claims = refresh_claims.with_email(e);
        }
        if let Some(u) = username {
            refresh_claims = refresh_claims.with_username(u);
        }
        if let Some(r) = roles {
            refresh_claims = refresh_claims.with_roles(r);
        }
        if let Some(s) = session_id {
            refresh_claims = refresh_claims.with_session_id(s);
        }

        let access_token = self.generate_access_token(access_claims)?;
        let refresh_token = self.generate_refresh_token(refresh_claims)?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer".into(),
            expires_in: self.config.access_token_expiry_minutes * 60,
            refresh_expires_in: self.config.refresh_token_expiry_days * 24 * 60 * 60,
            id_token: None,
            scope: None,
        })
    }

    pub fn validate_token(&self, token: &str) -> Result<TokenData<Claims>> {
        let mut validation = Validation::new(self.config.algorithm.to_jsonwebtoken());
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&[&self.config.audience]);
        validation.leeway = self.config.leeway_seconds;

        decode::<Claims>(token, &self.decoding_key, &validation)
            .map_err(|e| anyhow!("Token validation failed: {e}"))
    }

    pub async fn validate_token_with_blacklist(&self, token: &str) -> Result<TokenData<Claims>> {
        let token_data = self.validate_token(token)?;

        if self.blacklist.contains(&token_data.claims.jti).await {
            return Err(anyhow!("Token has been revoked"));
        }

        Ok(token_data)
    }

    pub fn validate_access_token(&self, token: &str) -> Result<Claims> {
        let token_data = self.validate_token(token)?;

        if !token_data.claims.is_access_token() {
            return Err(anyhow!("Token is not an access token"));
        }

        Ok(token_data.claims)
    }

    pub fn validate_refresh_token(&self, token: &str) -> Result<Claims> {
        let token_data = self.validate_token(token)?;

        if !token_data.claims.is_refresh_token() {
            return Err(anyhow!("Token is not a refresh token"));
        }

        Ok(token_data.claims)
    }

    pub async fn refresh_tokens(&self, refresh_token: &str) -> Result<TokenPair> {
        let claims = self.validate_refresh_token(refresh_token)?;

        if self.blacklist.contains(&claims.jti).await {
            return Err(anyhow!("Refresh token has been revoked"));
        }

        let user_id = claims.user_id()?;

        self.revoke_token_with_expiry(&claims.jti, claims.exp).await?;

        let new_pair = self.generate_token_pair_with_claims(
            user_id,
            claims.email,
            claims.username,
            claims.roles,
            claims.session_id,
        )?;

        info!("Tokens refreshed for user {}", user_id);
        Ok(new_pair)
    }

    /// Revokes a token by JTI. When the original expiration is unknown the
    /// entry is bounded by the refresh-token lifetime (the longest any token
    /// can be valid), so cleanup can still prune it once that window passes.
    pub async fn revoke_token(&self, jti: &str) -> Result<()> {
        let now = Utc::now();
        let expires_at = now + Duration::days(self.config.refresh_token_expiry_days);
        self.revoke_token_with_expiry(jti, expires_at.timestamp())
            .await
    }

    /// Revokes a token with its original `exp` claim so the entry can be
    /// pruned exactly when both revocation and expiration are in the past.
    pub async fn revoke_token_with_expiry(&self, jti: &str, expires_at_epoch: i64) -> Result<()> {
        let now = Utc::now();
        let expires_at = DateTime::<Utc>::from_timestamp(expires_at_epoch, 0)
            .unwrap_or_else(|| now + Duration::days(self.config.refresh_token_expiry_days));
        let entry = BlacklistEntry::new(jti.to_string(), now, expires_at);
        self.blacklist.insert(entry).await?;
        info!("Token revoked: {}", jti);
        Ok(())
    }

    pub async fn revoke_by_token(&self, token: &str) -> Result<()> {
        let token_data = self.validate_token(token)?;
        self.revoke_token_with_expiry(&token_data.claims.jti, token_data.claims.exp)
            .await
    }

    pub async fn is_revoked(&self, jti: &str) -> bool {
        self.blacklist.contains(jti).await
    }

    /// Prunes entries whose revocation and original expiration both precede
    /// `expired_before`. Returns the number of removed entries.
    pub async fn cleanup_blacklist(&self, expired_before: DateTime<Utc>) -> usize {
        let removed = self.blacklist.cleanup(expired_before).await;
        let remaining = self.blacklist.len().await;
        info!(
            "Token blacklist cleanup: removed {removed} entries, {} remain",
            remaining
        );
        removed
    }

    /// Current blacklist size — exposed as a metric so growth is observable.
    pub async fn blacklist_size(&self) -> usize {
        self.blacklist.len().await
    }

    pub fn decode_without_validation(&self, token: &str) -> Result<Claims> {
        let token_data = jsonwebtoken::dangerous::insecure_decode::<Claims>(&token)
            .map_err(|e| anyhow!("Failed to decode token: {e}"))?;

        Ok(token_data.claims)
    }

    pub fn config(&self) -> &JwtConfig {
        &self.config
    }
}

impl botlib::traits::JwtService for JwtManager {
    fn validate_access_token(&self, token: &str) -> Result<serde_json::Value, String> {
        let claims = JwtManager::validate_access_token(self, token)
            .map_err(|e| format!("validate_access_token: {e}"))?;
        serde_json::to_value(&claims)
            .map_err(|e| format!("serialize claims: {e}"))
            .map(|v| v)
    }

    fn generate_access_token(&self, user_id: uuid::Uuid, claims: serde_json::Value) -> Result<String, String> {
        let token_obj: Claims = serde_json::from_value(claims).map_err(|e| e.to_string())?;
        let expiry = DateTime::<Utc>::from_timestamp(token_obj.exp, 0)
            .unwrap_or_else(Utc::now);
        let mut new_claims =
            Claims::new(user_id, &token_obj.iss, &token_obj.aud, TokenType::Access, expiry)
                .map_err(|e| e.to_string())?;
        new_claims.email = token_obj.email;
        new_claims.username = token_obj.username;
        new_claims.roles = token_obj.roles;
        new_claims.permissions = token_obj.permissions;
        new_claims.session_id = token_obj.session_id;
        new_claims.organization_id = token_obj.organization_id;
        new_claims.org_id = token_obj.org_id;
        new_claims.branch_id = token_obj.branch_id;
        new_claims.device_id = token_obj.device_id;
        let header = Header::new(self.config.algorithm.to_jsonwebtoken());
        encode(
            &header,
            &new_claims,
            &self.encoding_key,
        )
        .map_err(|e| e.to_string())
    }
}
