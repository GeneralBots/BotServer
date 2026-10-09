use super::*;

impl PinterestProvider {
    pub async fn delete_board(
        &self,
        access_token: &str,
        board_id: &str,
    ) -> Result<(), ChannelError> {
        let url = format!("{}/boards/{}", Self::API_BASE, board_id);

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if response.status().as_u16() == 204 {
            return Ok(());
        }

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        Ok(())
    }

    /// List pins on a board
    pub async fn list_board_pins(
        &self,
        access_token: &str,
        board_id: &str,
        options: &PinListOptions,
    ) -> Result<PinListResponse, ChannelError> {
        let url = format!("{}/boards/{}/pins", Self::API_BASE, board_id);

        let mut query_params = vec![];

        if let Some(page_size) = options.page_size {
            query_params.push(("page_size", page_size.to_string()));
        }

        if let Some(bookmark) = &options.bookmark {
            query_params.push(("bookmark", bookmark.clone()));
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
            .json::<PinListResponse>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }

    /// Create a board section
    pub async fn create_board_section(
        &self,
        access_token: &str,
        board_id: &str,
        section: &BoardSectionCreateRequest,
    ) -> Result<BoardSection, ChannelError> {
        let url = format!("{}/boards/{}/sections", Self::API_BASE, board_id);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(section)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        response
            .json::<BoardSection>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }

    /// List board sections
    pub async fn list_board_sections(
        &self,
        access_token: &str,
        board_id: &str,
        options: &PaginationOptions,
    ) -> Result<BoardSectionListResponse, ChannelError> {
        let url = format!("{}/boards/{}/sections", Self::API_BASE, board_id);

        let mut query_params = vec![];

        if let Some(page_size) = options.page_size {
            query_params.push(("page_size", page_size.to_string()));
        }

        if let Some(bookmark) = &options.bookmark {
            query_params.push(("bookmark", bookmark.clone()));
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
            .json::<BoardSectionListResponse>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }

    /// Search pins
    pub async fn search_pins(
        &self,
        access_token: &str,
        query: &str,
        options: &SearchOptions,
    ) -> Result<PinSearchResponse, ChannelError> {
        let url = format!("{}/search/pins", Self::API_BASE);

        let mut query_params = vec![("query", query.to_string())];

        if let Some(page_size) = options.page_size {
            query_params.push(("page_size", page_size.to_string()));
        }

        if let Some(bookmark) = &options.bookmark {
            query_params.push(("bookmark", bookmark.clone()));
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
            .json::<PinSearchResponse>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }

    // ========================================================================
    // Advertising API
    // ========================================================================

    /// List ad accounts
    pub async fn list_ad_accounts(
        &self,
        access_token: &str,
        options: &PaginationOptions,
    ) -> Result<AdAccountListResponse, ChannelError> {
        let url = format!("{}/ad_accounts", Self::API_BASE);

        let mut query_params = vec![];

        if let Some(page_size) = options.page_size {
            query_params.push(("page_size", page_size.to_string()));
        }

        if let Some(bookmark) = &options.bookmark {
            query_params.push(("bookmark", bookmark.clone()));
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
            .json::<AdAccountListResponse>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }

    /// Create a campaign
    pub async fn create_campaign(
        &self,
        access_token: &str,
        ad_account_id: &str,
        campaign: &CampaignCreateRequest,
    ) -> Result<Campaign, ChannelError> {
        let url = format!(
            "{}/ad_accounts/{}/campaigns",
            Self::API_BASE, ad_account_id
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(campaign)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        response
            .json::<Campaign>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }
}
