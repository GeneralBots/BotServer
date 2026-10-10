use super::*;

impl ZitadelAuthProvider {
    pub async fn get_service_token(&self) -> Result<String, AuthError> {
        {
            let token = self.service_token.read().await;
            if let Some(ref t) = *token {
                if t.expires_at > chrono::Utc::now().timestamp() {
                    return Ok(t.access_token.clone());
                }
            }
        }

        let token_url = format!("{}/oauth/v2/token", self.config.sanitized_api_base().unwrap_or_default());

        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", &self.config.client_id),
            ("client_secret", &self.config.client_secret),
            ("scope", "openid profile email"),
        ];

        let response = self
            .http_client
            .post(&token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                log::error!("Service token request failed: {}", e);
                AuthError::InternalError("Authentication service unavailable".to_string())
            })?;

        if !response.status().is_success() {
            return Err(AuthError::InternalError(
                "Failed to obtain service token".to_string(),
            ));
        }

        let token_data: serde_json::Value = response.json().await.map_err(|e| {
            log::error!("Failed to parse token response: {}", e);
            AuthError::InternalError("Invalid token response".to_string())
        })?;

        let access_token = token_data
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AuthError::InternalError("No access token in response".to_string()))?
            .to_string();

        let expires_in = token_data
            .get("expires_in")
            .and_then(|v| v.as_i64())
            .unwrap_or(3600);

        let expires_at = chrono::Utc::now().timestamp() + expires_in - 60;

        {
            let mut token = self.service_token.write().await;
            *token = Some(ServiceToken {
                access_token: access_token.clone(),
                expires_at,
            });
        }

        Ok(access_token)
    }

    pub async fn get_cached_user(&self, key: &str) -> Option<AuthenticatedUser> {
        let cache = self.user_cache.read().await;
        cache.get(key).and_then(|cached| {
            if cached.expires_at > chrono::Utc::now().timestamp() {
                Some(cached.user.clone())
            } else {
                None
            }
        })
    }

    pub async fn cache_user(&self, key: &str, user: &AuthenticatedUser) {
        let expires_at = chrono::Utc::now().timestamp() + self.config.cache_ttl_secs as i64;
        let cached = CachedUser {
            user: user.clone(),
            expires_at,
        };

        let mut cache = self.user_cache.write().await;
        cache.insert(key.to_string(), cached);
    }

    pub async fn clear_cache(&self) {
        let mut cache = self.user_cache.write().await;
        cache.clear();
    }

    pub async fn invalidate_user(&self, token: &str) {
        let mut cache = self.user_cache.write().await;
        cache.remove(token);
    }

    pub async fn get_user_bot_access(
        &self,
        user_id: &str,
    ) -> Result<Vec<BotAccess>, AuthError> {
        let service_token = self.get_service_token().await?;

        let url = format!(
            "{}/v2/users/{}/grants",
            self.config.sanitized_api_base().unwrap_or_default(), user_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&service_token)
            .send()
            .await
            .map_err(|e| {
                log::error!("Failed to get user grants: {}", e);
                AuthError::InternalError("Failed to fetch user permissions".to_string())
            })?;

        if !response.status().is_success() {
            return Ok(Vec::new());
        }

        let grants: serde_json::Value = response.json().await.map_err(|e| {
            log::error!("Failed to parse grants response: {}", e);
            AuthError::InternalError("Invalid grants response".to_string())
        })?;

        let mut bot_access = Vec::new();

        if let Some(results) = grants.get("result").and_then(|r| r.as_array()) {
            for grant in results {
                if let Some(roles) = grant.get("roles").and_then(|r| r.as_array()) {
                    for role_value in roles {
                        if let Some(role_str) = role_value.as_str() {
                            if role_str.starts_with("bot:") {
                                let parts: Vec<&str> = role_str.splitn(3, ':').collect();
                                if parts.len() >= 2 {
                                    if let Ok(bot_id) = Uuid::parse_str(parts[1]) {
                                        let role = if parts.len() >= 3 {
                                            map_zitadel_role_to_role(parts[2])
                                        } else {
                                            Role::BotViewer
                                        };

                                        bot_access.push(BotAccess::new(bot_id, role));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(bot_access)
    }

    pub async fn check_bot_permission(
        &self,
        user_id: &str,
        bot_id: &Uuid,
        permission: &Permission,
    ) -> Result<bool, AuthError> {
        let bot_access = self.get_user_bot_access(user_id).await?;

        for access in bot_access {
            if &access.bot_id == bot_id && access.role.has_permission(permission) {
                return Ok(true);
            }
        }

        Ok(false)
    }
}
