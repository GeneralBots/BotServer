use super::*;

impl TikTokProvider {
    pub async fn query_videos(
        &self,
        access_token: &str,
        video_ids: &[String],
        fields: Option<&str>,
    ) -> Result<Vec<TikTokVideo>, ChannelError> {
        let url = format!("{}/video/query/", self.api_base_url);

        let fields_param = fields.unwrap_or(
            "id,create_time,cover_image_url,share_url,video_description,duration,title,like_count,comment_count,share_count,view_count",
        );

        let request_body = serde_json::json!({
            "filters": {
                "video_ids": video_ids
            }
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json; charset=UTF-8")
            .query(&[("fields", fields_param)])
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        let api_response: TikTokApiResponse<VideoQueryResponse> =
            response.json().await.map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })?;

        api_response
            .data
            .map(|d| d.videos)
            .ok_or_else(|| ChannelError::ApiError {
                code: api_response.error.as_ref().map(|e| e.code.clone()),
                message: api_response
                    .error
                    .map(|e| e.message)
                    .unwrap_or_else(|| "Failed to query videos".to_string()),
            })
    }

    /// Get creator info for content posting (requires video.publish scope)
    pub async fn get_creator_info(
        &self,
        access_token: &str,
    ) -> Result<CreatorInfo, ChannelError> {
        let url = format!("{}/post/publish/creator_info/", self.api_base_url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        let api_response: TikTokApiResponse<CreatorInfo> =
            response.json().await.map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })?;

        api_response.data.ok_or_else(|| ChannelError::ApiError {
            code: api_response.error.as_ref().map(|e| e.code.clone()),
            message: api_response
                .error
                .map(|e| e.message)
                .unwrap_or_else(|| "Failed to get creator info".to_string()),
        })
    }

    /// Refresh OAuth access token
    pub async fn refresh_oauth_token(
        &self,
        client_key: &str,
        client_secret: &str,
        refresh_token: &str,
    ) -> Result<OAuthTokenResponse, ChannelError> {
        let url = format!("{}/token/", self.oauth_base_url);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&[
                ("client_key", client_key),
                ("client_secret", client_secret),
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
            ])
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        response
            .json::<OAuthTokenResponse>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }

    /// Revoke OAuth access token
    pub async fn revoke_token(
        &self,
        client_key: &str,
        client_secret: &str,
        access_token: &str,
    ) -> Result<(), ChannelError> {
        let url = format!("{}/revoke/", self.oauth_base_url);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&[
                ("client_key", client_key),
                ("client_secret", client_secret),
                ("token", access_token),
            ])
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        Ok(())
    }

    /// Generate OAuth authorization URL
    pub fn get_authorization_url(
        &self,
        client_key: &str,
        redirect_uri: &str,
        scope: &str,
        state: &str,
    ) -> String {
        format!(
            "https://www.tiktok.com/v2/auth/authorize/?client_key={}&scope={}&response_type=code&redirect_uri={}&state={}",
            client_key,
            urlencoding::encode(scope),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(state)
        )
    }

    /// Exchange authorization code for access token
    pub async fn exchange_code_for_token(
        &self,
        client_key: &str,
        client_secret: &str,
        code: &str,
        redirect_uri: &str,
    ) -> Result<OAuthTokenResponse, ChannelError> {
        let url = format!("{}/token/", self.oauth_base_url);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&[
                ("client_key", client_key),
                ("client_secret", client_secret),
                ("code", code),
                ("grant_type", "authorization_code"),
                ("redirect_uri", redirect_uri),
            ])
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        response
            .json::<OAuthTokenResponse>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }

    pub(crate) async fn parse_error_response(&self, response: reqwest::Response) -> ChannelError {
        let status = response.status();

        if status.as_u16() == 401 {
            return ChannelError::AuthenticationFailed("Invalid or expired token".to_string());
        }

        if status.as_u16() == 429 {
            let retry_after = response
                .headers()
                .get("x-ratelimit-reset")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok());
            return ChannelError::RateLimited { retry_after };
        }

        let error_text = response.text().await.unwrap_or_default();

        if let Ok(api_response) = serde_json::from_str::<TikTokApiResponse<()>>(&error_text) {
            if let Some(error) = api_response.error {
                return ChannelError::ApiError {
                    code: Some(error.code),
                    message: error.message,
                };
            }
        }

        ChannelError::ApiError {
            code: Some(status.to_string()),
            message: error_text,
        }
    }
}
