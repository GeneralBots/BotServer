use super::*;

impl PinterestProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Get authenticated user info
    pub async fn get_user(&self, access_token: &str) -> Result<PinterestUser, ChannelError> {
        let url = format!("{}/user_account", Self::API_BASE);

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

        response
            .json::<PinterestUser>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }

    /// Create a new pin
    pub async fn create_pin(
        &self,
        access_token: &str,
        pin: &PinCreateRequest,
    ) -> Result<Pin, ChannelError> {
        let url = format!("{}/pins", Self::API_BASE);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(pin)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        response.json::<Pin>().await.map_err(|e| ChannelError::ApiError {
            code: None,
            message: e.to_string(),
        })
    }

    /// Get a pin by ID
    pub async fn get_pin(
        &self,
        access_token: &str,
        pin_id: &str,
    ) -> Result<Pin, ChannelError> {
        let url = format!("{}/pins/{}", Self::API_BASE, pin_id);

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

        response.json::<Pin>().await.map_err(|e| ChannelError::ApiError {
            code: None,
            message: e.to_string(),
        })
    }

    /// Delete a pin
    pub async fn delete_pin(
        &self,
        access_token: &str,
        pin_id: &str,
    ) -> Result<(), ChannelError> {
        let url = format!("{}/pins/{}", Self::API_BASE, pin_id);

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

    /// Save a pin to a board
    pub async fn save_pin(
        &self,
        access_token: &str,
        pin_id: &str,
        board_id: &str,
        board_section_id: Option<&str>,
    ) -> Result<Pin, ChannelError> {
        let url = format!("{}/pins/{}/save", Self::API_BASE, pin_id);

        let mut request_body = serde_json::json!({
            "board_id": board_id
        });

        if let Some(section_id) = board_section_id {
            request_body["board_section_id"] = serde_json::json!(section_id);
        }

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

        response.json::<Pin>().await.map_err(|e| ChannelError::ApiError {
            code: None,
            message: e.to_string(),
        })
    }

    /// Create a new board
    pub async fn create_board(
        &self,
        access_token: &str,
        board: &BoardCreateRequest,
    ) -> Result<Board, ChannelError> {
        let url = format!("{}/boards", Self::API_BASE);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(board)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        response
            .json::<Board>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }

    /// List user's boards
    pub async fn list_boards(
        &self,
        access_token: &str,
        options: &BoardListOptions,
    ) -> Result<BoardListResponse, ChannelError> {
        let url = format!("{}/boards", Self::API_BASE);

        let mut query_params = vec![];

        if let Some(page_size) = options.page_size {
            query_params.push(("page_size", page_size.to_string()));
        }

        if let Some(bookmark) = &options.bookmark {
            query_params.push(("bookmark", bookmark.clone()));
        }

        if let Some(privacy) = &options.privacy {
            query_params.push(("privacy", privacy.clone()));
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
            .json::<BoardListResponse>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }

    /// Get a board by ID
    pub async fn get_board(
        &self,
        access_token: &str,
        board_id: &str,
    ) -> Result<Board, ChannelError> {
        let url = format!("{}/boards/{}", Self::API_BASE, board_id);

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

        response
            .json::<Board>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }

    /// Update a board
    pub async fn update_board(
        &self,
        access_token: &str,
        board_id: &str,
        update: &BoardUpdateRequest,
    ) -> Result<Board, ChannelError> {
        let url = format!("{}/boards/{}", Self::API_BASE, board_id);

        let response = self
            .client
            .patch(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(update)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(self.parse_error_response(response).await);
        }

        response
            .json::<Board>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }
}
