use super::*;

impl RedditChannel {
    pub fn new(config: RedditConfig) -> Self {
        let http_client = Client::builder()
            .user_agent(&config.user_agent)
            .build()
            .unwrap_or_default();

        Self {
            config,
            http_client,
            tokens: Arc::new(RwLock::new(None)),
            base_url: "https://oauth.reddit.com".to_string(),
            oauth_url: "https://www.reddit.com/api/v1".to_string(),
        }
    }

    pub fn get_authorization_url(&self, state: &str, scopes: &[&str]) -> String {
        let scope = scopes.join(" ");
        format!(
            "{}/authorize?client_id={}&response_type=code&state={}&redirect_uri={}&duration=permanent&scope={}",
            self.oauth_url,
            self.config.client_id,
            state,
            urlencoding::encode(&self.config.redirect_uri),
            urlencoding::encode(&scope)
        )
    }

    pub async fn exchange_code(&self, code: &str) -> Result<RedditTokens, RedditError> {
        let response = self
            .http_client
            .post(format!("{}/access_token", self.oauth_url))
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", &self.config.redirect_uri),
            ])
            .send()
            .await
            .map_err(|e| RedditError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(RedditError::ApiError(error_text));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| RedditError::ParseError(e.to_string()))?;

        let tokens = RedditTokens {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_at: Utc::now() + chrono::Duration::seconds(token_response.expires_in as i64),
            scope: token_response.scope,
        };

        let mut token_guard = self.tokens.write().await;
        *token_guard = Some(tokens.clone());

        Ok(tokens)
    }

    pub async fn refresh_token(&self) -> Result<RedditTokens, RedditError> {
        let tokens = self.tokens.read().await;
        let refresh_token = tokens
            .as_ref()
            .and_then(|t| t.refresh_token.clone())
            .ok_or(RedditError::NotAuthenticated)?;
        drop(tokens);

        let response = self
            .http_client
            .post(format!("{}/access_token", self.oauth_url))
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", &refresh_token),
            ])
            .send()
            .await
            .map_err(|e| RedditError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(RedditError::ApiError(error_text));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| RedditError::ParseError(e.to_string()))?;

        let new_tokens = RedditTokens {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token.or(Some(refresh_token)),
            expires_at: Utc::now() + chrono::Duration::seconds(token_response.expires_in as i64),
            scope: token_response.scope,
        };

        let mut token_guard = self.tokens.write().await;
        *token_guard = Some(new_tokens.clone());

        Ok(new_tokens)
    }

    pub async fn authenticate_script(&self) -> Result<RedditTokens, RedditError> {
        let username = self
            .config
            .username
            .as_ref()
            .ok_or(RedditError::ConfigError("Username required for script auth".to_string()))?;
        let password = self
            .config
            .password
            .as_ref()
            .ok_or(RedditError::ConfigError("Password required for script auth".to_string()))?;

        let response = self
            .http_client
            .post(format!("{}/access_token", self.oauth_url))
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .form(&[
                ("grant_type", "password"),
                ("username", username.as_str()),
                ("password", password.as_str()),
            ])
            .send()
            .await
            .map_err(|e| RedditError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(RedditError::ApiError(error_text));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| RedditError::ParseError(e.to_string()))?;

        let tokens = RedditTokens {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            expires_at: Utc::now() + chrono::Duration::seconds(token_response.expires_in as i64),
            scope: token_response.scope,
        };

        let mut token_guard = self.tokens.write().await;
        *token_guard = Some(tokens.clone());

        Ok(tokens)
    }

    async fn get_access_token(&self) -> Result<String, RedditError> {
        let tokens = self.tokens.read().await;
        match tokens.as_ref() {
            Some(t) if t.expires_at > Utc::now() => Ok(t.access_token.clone()),
            Some(_) => {
                drop(tokens);
                let new_tokens = self.refresh_token().await?;
                Ok(new_tokens.access_token)
            }
            None => Err(RedditError::NotAuthenticated),
        }
    }

    pub async fn get_me(&self) -> Result<RedditUser, RedditError> {
        let token = self.get_access_token().await?;

        let response = self
            .http_client
            .get(format!("{}/api/v1/me", self.base_url))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RedditError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(RedditError::ApiError(error_text));
        }

        let user: RedditUser = response
            .json()
            .await
            .map_err(|e| RedditError::ParseError(e.to_string()))?;

        Ok(user)
    }

    pub async fn submit_post(&self, request: SubmitPostRequest) -> Result<RedditPost, RedditError> {
        let token = self.get_access_token().await?;

        let kind = match request.kind {
            PostKind::Text => "self",
            PostKind::Link => "link",
            PostKind::Image => "image",
            PostKind::Video => "video",
            PostKind::Poll => "poll",
        };

        let mut params: HashMap<&str, String> = HashMap::new();
        params.insert("sr", request.subreddit.clone());
        params.insert("title", request.title.clone());
        params.insert("kind", kind.to_string());
        params.insert("api_type", "json".to_string());
        params.insert("send_replies", request.send_replies.to_string());
        params.insert("nsfw", request.nsfw.to_string());
        params.insert("spoiler", request.spoiler.to_string());

        match request.kind {
            PostKind::Text => {
                params.insert("text", request.content.clone());
            }
            PostKind::Link | PostKind::Image | PostKind::Video => {
                params.insert("url", request.content.clone());
            }
            PostKind::Poll => {
                params.insert("text", request.content.clone());
            }
        }

        if let Some(flair_id) = &request.flair_id {
            params.insert("flair_id", flair_id.clone());
        }
        if let Some(flair_text) = &request.flair_text {
            params.insert("flair_text", flair_text.clone());
        }

        let response = self
            .http_client
            .post(format!("{}/api/submit", self.base_url))
            .bearer_auth(&token)
            .form(&params)
            .send()
            .await
            .map_err(|e| RedditError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(RedditError::ApiError(error_text));
        }

        let submit_response: SubmitResponse = response
            .json()
            .await
            .map_err(|e| RedditError::ParseError(e.to_string()))?;

        if let Some(errors) = submit_response.json.errors {
            if !errors.is_empty() {
                let error_msg = errors
                    .iter()
                    .map(|e| e.join(": "))
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(RedditError::ApiError(error_msg));
            }
        }

        let post_id = submit_response
            .json
            .data
            .ok_or_else(|| RedditError::ApiError("No post ID returned".to_string()))?
            .id;

        self.get_post(&post_id).await
    }
}
