use super::*;

impl ApiKeyManager {
    pub fn new(config: ApiKeyConfig) -> Self {
        Self {
            config,
            keys: Arc::new(RwLock::new(HashMap::new())),
            key_hashes: Arc::new(RwLock::new(HashMap::new())),
            usage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(ApiKeyConfig::default())
    }

    pub async fn create_key(
        &self,
        user_id: Uuid,
        request: CreateApiKeyRequest,
    ) -> Result<CreateApiKeyResponse> {
        let user_keys = self.get_user_keys(user_id).await;
        if user_keys.len() >= self.config.max_keys_per_user {
            return Err(anyhow!(
                "Maximum number of API keys ({}) reached",
                self.config.max_keys_per_user
            ));
        }

        let (secret, key_hash) = generate_api_key();
        let key_prefix = extract_prefix(&secret);

        let expires_at = request
            .expires_in_days
            .or(self.config.key_expiry_days)
            .map(|days| Utc::now() + Duration::days(days as i64));

        let rate_limits = request.rate_limits.unwrap_or(RateLimitConfig {
            requests_per_minute: self.config.default_rate_limit_per_minute,
            requests_per_hour: self.config.default_rate_limit_per_hour,
            requests_per_day: self.config.default_rate_limit_per_day,
        });

        let key = ApiKey {
            id: Uuid::new_v4(),
            user_id,
            name: request.name,
            description: request.description,
            key_hash: key_hash.clone(),
            key_prefix,
            scopes: request.scopes.into_iter().collect(),
            status: ApiKeyStatus::Active,
            rate_limits,
            allowed_ips: request.allowed_ips,
            allowed_origins: request.allowed_origins,
            created_at: Utc::now(),
            expires_at,
            last_used_at: None,
            rotated_from: None,
            rotation_deadline: None,
            metadata: request.metadata.unwrap_or_default(),
        };

        let key_id = key.id;

        {
            let mut keys = self.keys.write().await;
            keys.insert(key_id, key.clone());
        }

        {
            let mut hashes = self.key_hashes.write().await;
            hashes.insert(key_hash, key_id);
        }

        info!("Created API key {} for user {}", key_id, user_id);

        Ok(CreateApiKeyResponse { key, secret })
    }

    pub async fn validate_key(&self, secret: &str) -> Result<Option<ApiKey>> {
        let key_hash = hash_api_key(secret);

        let key_id = {
            let hashes = self.key_hashes.read().await;
            hashes.get(&key_hash).copied()
        };

        let key_id = match key_id {
            Some(id) => id,
            None => return Ok(None),
        };

        let key = {
            let keys = self.keys.read().await;
            keys.get(&key_id).cloned()
        };

        let key = match key {
            Some(k) => k,
            None => return Ok(None),
        };

        if !key.is_valid() {
            return Ok(None);
        }

        {
            let mut keys = self.keys.write().await;
            if let Some(k) = keys.get_mut(&key_id) {
                k.last_used_at = Some(Utc::now());
            }
        }

        Ok(Some(key))
    }

    pub async fn validate_and_check_rate_limit(&self, secret: &str) -> Result<(ApiKey, bool)> {
        let key = self
            .validate_key(secret)
            .await?
            .ok_or_else(|| anyhow!("Invalid API key"))?;

        let is_rate_limited = {
            let mut usage = self.usage.write().await;
            let key_usage = usage
                .entry(key.id)
                .or_insert_with(|| ApiKeyUsage::new(key.id));

            if key_usage.is_rate_limited(&key.rate_limits) {
                true
            } else {
                key_usage.record_request();
                false
            }
        };

        Ok((key, is_rate_limited))
    }

    pub async fn get_key(&self, key_id: Uuid) -> Option<ApiKey> {
        let keys = self.keys.read().await;
        keys.get(&key_id).cloned()
    }

    pub async fn get_user_keys(&self, user_id: Uuid) -> Vec<ApiKey> {
        let keys = self.keys.read().await;
        keys.values()
            .filter(|k| k.user_id == user_id)
            .cloned()
            .collect()
    }

    pub async fn revoke_key(&self, key_id: Uuid) -> Result<bool> {
        let mut keys = self.keys.write().await;

        if let Some(key) = keys.get_mut(&key_id) {
            key.status = ApiKeyStatus::Revoked;
            info!("Revoked API key {}", key_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn revoke_all_user_keys(&self, user_id: Uuid) -> Result<usize> {
        let mut keys = self.keys.write().await;
        let mut revoked = 0;

        for key in keys.values_mut() {
            if key.user_id == user_id && key.status == ApiKeyStatus::Active {
                key.status = ApiKeyStatus::Revoked;
                revoked += 1;
            }
        }

        info!("Revoked {} API keys for user {}", revoked, user_id);
        Ok(revoked)
    }

    pub async fn rotate_key(&self, key_id: Uuid) -> Result<CreateApiKeyResponse> {
        if !self.config.allow_key_rotation {
            return Err(anyhow!("API key rotation is not enabled"));
        }

        let old_key = {
            let keys = self.keys.read().await;
            keys.get(&key_id).cloned()
        };

        let old_key = old_key.ok_or_else(|| anyhow!("API key not found"))?;

        if old_key.status != ApiKeyStatus::Active {
            return Err(anyhow!("Can only rotate active keys"));
        }

        let (secret, key_hash) = generate_api_key();
        let key_prefix = extract_prefix(&secret);
        let grace_period = Duration::hours(self.config.rotation_grace_period_hours as i64);

        let new_key = ApiKey {
            id: Uuid::new_v4(),
            user_id: old_key.user_id,
            name: old_key.name.clone(),
            description: old_key.description.clone(),
            key_hash: key_hash.clone(),
            key_prefix,
            scopes: old_key.scopes.clone(),
            status: ApiKeyStatus::Active,
            rate_limits: old_key.rate_limits.clone(),
            allowed_ips: old_key.allowed_ips.clone(),
            allowed_origins: old_key.allowed_origins.clone(),
            created_at: Utc::now(),
            expires_at: old_key.expires_at,
            last_used_at: None,
            rotated_from: Some(key_id),
            rotation_deadline: None,
            metadata: old_key.metadata.clone(),
        };

        let new_key_id = new_key.id;

        {
            let mut keys = self.keys.write().await;

            if let Some(old) = keys.get_mut(&key_id) {
                old.status = ApiKeyStatus::Rotating;
                old.rotation_deadline = Some(Utc::now() + grace_period);
            }

            keys.insert(new_key_id, new_key.clone());
        }

        {
            let mut hashes = self.key_hashes.write().await;
            hashes.insert(key_hash, new_key_id);
        }

        info!(
            "Rotated API key {} to {} (grace period: {} hours)",
            key_id, new_key_id, self.config.rotation_grace_period_hours
        );

        Ok(CreateApiKeyResponse {
            key: new_key,
            secret,
        })
    }

    pub async fn update_key_scopes(&self, key_id: Uuid, scopes: Vec<ApiKeyScope>) -> Result<bool> {
        let mut keys = self.keys.write().await;

        if let Some(key) = keys.get_mut(&key_id) {
            key.scopes = scopes.into_iter().collect();
            info!("Updated scopes for API key {}", key_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn update_key_rate_limits(
        &self,
        key_id: Uuid,
        rate_limits: RateLimitConfig,
    ) -> Result<bool> {
        let mut keys = self.keys.write().await;

        if let Some(key) = keys.get_mut(&key_id) {
            key.rate_limits = rate_limits;
            info!("Updated rate limits for API key {}", key_id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn get_key_usage(&self, key_id: Uuid) -> Option<ApiKeyUsage> {
        let usage = self.usage.read().await;
        usage.get(&key_id).cloned()
    }
}
