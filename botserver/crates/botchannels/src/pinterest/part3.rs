use super::*;

impl PinterestProvider {
    pub async fn list_campaigns(
        &self,
        access_token: &str,
        ad_account_id: &str,
        options: &CampaignListOptions,
    ) -> Result<CampaignListResponse, ChannelError> {
        let url = format!(
            "{}/ad_accounts/{}/campaigns",
            Self::API_BASE, ad_account_id
        );

        let mut query_params = vec![];

        if let Some(page_size) = options.page_size {
            query_params.push(("page_size", page_size.to_string()));
        }

        if let Some(bookmark) = &options.bookmark {
            query_params.push(("bookmark", bookmark.clone()));
        }

        if let Some(order) = &options.order {
            query_params.push(("order", order.clone()));
        }

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .query(&query_params)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        response
            .json::<CampaignListResponse>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }

    /// Create an ad group
    pub async fn create_ad_group(
        &self,
        access_token: &str,
        ad_account_id: &str,
        ad_group: &AdGroupCreateRequest,
    ) -> Result<AdGroup, ChannelError> {
        let url = format!(
            "{}/ad_accounts/{}/ad_groups",
            Self::API_BASE, ad_account_id
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(ad_group)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        response
            .json::<AdGroup>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }

    /// Create an ad (Promoted Pin)
    pub async fn create_ad(
        &self,
        access_token: &str,
        ad_account_id: &str,
        ad: &AdCreateRequest,
    ) -> Result<Ad, ChannelError> {
        let url = format!("{}/ad_accounts/{}/ads", Self::API_BASE, ad_account_id);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(ad)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        response.json::<Ad>().await.map_err(|e| ChannelError::ApiError {
            code: None,
            message: e.to_string(),
        })
    }

    /// Get analytics for a pin
    pub async fn get_pin_analytics(
        &self,
        access_token: &str,
        pin_id: &str,
        start_date: &str,
        end_date: &str,
        metric_types: &[&str],
    ) -> Result<PinAnalytics, ChannelError> {
        let url = format!("{}/pins/{}/analytics", Self::API_BASE, pin_id);

        let metrics = metric_types.join(",");

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .query(&[
                ("start_date", start_date),
                ("end_date", end_date),
                ("metric_types", &metrics),
                ("app_types", "ALL"),
                ("split_field", "NO_SPLIT"),
            ])
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        response
            .json::<PinAnalytics>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }

    /// Get user account analytics
    pub async fn get_user_analytics(
        &self,
        access_token: &str,
        start_date: &str,
        end_date: &str,
        metric_types: &[&str],
    ) -> Result<UserAnalytics, ChannelError> {
        let url = format!("{}/user_account/analytics", Self::API_BASE);

        let metrics = metric_types.join(",");

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .query(&[
                ("start_date", start_date),
                ("end_date", end_date),
                ("metric_types", &metrics),
                ("app_types", "ALL"),
                ("split_field", "NO_SPLIT"),
            ])
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        response
            .json::<UserAnalytics>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }

    // ========================================================================
    // OAuth
    // ========================================================================

    /// Refresh OAuth token
    pub async fn refresh_oauth_token(
        &self,
        client_id: &str,
        client_secret: &str,
        refresh_token: &str,
    ) -> Result<OAuthTokenResponse, ChannelError> {
        let url = format!("{}/token", Self::OAUTH_BASE);

        let credentials = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            format!("{}:{}", client_id, client_secret),
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Basic {}", credentials))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&[
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

    /// Generate OAuth authorization URL
    pub fn get_authorization_url(
        &self,
        client_id: &str,
        redirect_uri: &str,
        scope: &str,
        state: &str,
    ) -> String {
        format!(
            "https://www.pinterest.com/oauth/?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
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
        let url = format!("{}/token", Self::OAUTH_BASE);

        let credentials = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            format!("{}:{}", client_id, client_secret),
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Basic {}", credentials))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
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
}
