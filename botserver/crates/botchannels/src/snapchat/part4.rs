use super::*;

impl SnapchatProvider {
    pub async fn create_audience(
        &self,
        access_token: &str,
        ad_account_id: &str,
        audience: &AudienceCreateRequest,
    ) -> Result<Audience, ChannelError> {
        let url = format!(
            "{}/adaccounts/{}/segments",
            Self::API_BASE, ad_account_id
        );

        let request_body = serde_json::json!({
            "segments": [{
                "name": audience.name,
                "description": audience.description,
                "source_type": audience.source_type,
                "retention_in_days": audience.retention_in_days,
                "ad_account_id": ad_account_id
            }]
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        let result: AudiencesResponse =
            response.json().await.map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })?;

        result
            .segments
            .into_iter()
            .next()
            .and_then(|wrapper| wrapper.segment)
            .ok_or_else(|| ChannelError::ApiError {
                code: None,
                message: "No audience in response".to_string(),
            })
    }

    /// Get campaign stats
    pub async fn get_campaign_stats(
        &self,
        access_token: &str,
        campaign_id: &str,
        granularity: &str,
        start_time: &str,
        end_time: &str,
    ) -> Result<CampaignStats, ChannelError> {
        let url = format!("{}/campaigns/{}/stats", Self::API_BASE, campaign_id);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .query(&[
                ("granularity", granularity),
                ("start_time", start_time),
                ("end_time", end_time),
            ])
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        response
            .json::<CampaignStats>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }

    /// Refresh OAuth token
    pub async fn refresh_oauth_token(
        &self,
        client_id: &str,
        client_secret: &str,
        refresh_token: &str,
    ) -> Result<OAuthTokenResponse, ChannelError> {
        let url = format!("{}/login/oauth2/access_token", Self::OAUTH_BASE);

        let response = self
            .client
            .post(&url)
            .form(&[
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("refresh_token", refresh_token),
                ("grant_type", "refresh_token"),
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

    /// Generate OAuth authorization URL
    pub fn get_authorization_url(
        &self,
        client_id: &str,
        redirect_uri: &str,
        scope: &str,
        state: &str,
    ) -> String {
        format!(
            "{}/login/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
            Self::OAUTH_BASE,
            client_id,
            urlencoding::encode(redirect_uri),
            urlencoding::encode(scope),
            urlencoding::encode(state)
        )
    }

    /// Exchange authorization code for access token
    pub async fn exchange_code_for_token(
        &self,
        client_id: &str,
        client_secret: &str,
        code: &str,
        redirect_uri: &str,
    ) -> Result<OAuthTokenResponse, ChannelError> {
        let url = format!("{}/login/oauth2/access_token", Self::OAUTH_BASE);

        let response = self
            .client
            .post(&url)
            .form(&[
                ("client_id", client_id),
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
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok());
            return ChannelError::RateLimited { retry_after };
        }

        let error_text = response.text().await.unwrap_or_default();

        if let Ok(error_response) = serde_json::from_str::<SnapchatErrorResponse>(&error_text) {
            return ChannelError::ApiError {
                code: error_response.request_status.clone(),
                message: error_response
                    .debug_message
                    .or(error_response.display_message)
                    .unwrap_or_else(|| error_response.request_status.unwrap_or_default()),
            };
        }

        ChannelError::ApiError {
            code: Some(status.to_string()),
            message: error_text,
        }
    }
}
