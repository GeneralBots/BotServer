use super::*;

impl Default for PinterestProvider {
    fn default() -> Self {
        Self::new()
    }
}
#[async_trait::async_trait]
impl ChannelProvider for PinterestProvider {
    fn channel_type(&self) -> ChannelType {
        ChannelType::Pinterest
    }

    fn max_text_length(&self) -> usize {
        500 // Pin description limit
    }

    fn supports_images(&self) -> bool {
        true
    }

    fn supports_video(&self) -> bool {
        true
    }

    fn supports_links(&self) -> bool {
        true
    }

    async fn post(
        &self,
        account: &ChannelAccount,
        content: &PostContent,
    ) -> Result<PostResult, ChannelError> {
        let access_token = match &account.credentials {
            ChannelCredentials::OAuth { access_token, .. } => access_token.clone(),
            _ => {
                return Err(ChannelError::AuthenticationFailed(
                    "OAuth credentials required for Pinterest".to_string(),
                ))
            }
        };

        let text = content.text.as_deref().unwrap_or("");

        if text.len() > self.max_text_length() {
            return Err(ChannelError::ContentTooLong {
                max_length: self.max_text_length(),
                actual_length: text.len(),
            });
        }

        // Get board_id from settings or metadata
        let board_id = content
            .metadata
            .get("board_id")
            .and_then(|v| v.as_str())
            .or_else(|| {
                account
                    .settings
                    .custom
                    .get("default_board_id")
                    .and_then(|v| v.as_str())
            })
            .ok_or_else(|| ChannelError::ApiError {
                code: None,
                message: "board_id required for Pinterest pin".to_string(),
            })?;

        // Build pin request
        let media_source = if let Some(image_url) = content.image_urls.first() {
            MediaSource::ImageUrl {
                url: image_url.clone(),
            }
        } else if let Some(video_url) = &content.video_url {
            MediaSource::VideoId {
                cover_image_url: content
                    .metadata
                    .get("cover_image_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                media_id: video_url.clone(),
            }
        } else {
            return Err(ChannelError::ApiError {
                code: None,
                message: "Pinterest requires an image or video".to_string(),
            });
        };

        let pin_request = PinCreateRequest {
            board_id: board_id.to_string(),
            title: content
                .metadata
                .get("title")
                .and_then(|v| v.as_str())
                .map(String::from),
            description: Some(text.to_string()),
            link: content.link.clone(),
            alt_text: content
                .metadata
                .get("alt_text")
                .and_then(|v| v.as_str())
                .map(String::from),
            board_section_id: content
                .metadata
                .get("board_section_id")
                .and_then(|v| v.as_str())
                .map(String::from),
            media_source,
        };

        let pin = self.create_pin(&access_token, &pin_request).await?;

        let url = format!("https://www.pinterest.com/pin/{}/", pin.id);

        Ok(PostResult::success(ChannelType::Pinterest, pin.id, Some(url)))
    }

    async fn validate_credentials(
        &self,
        credentials: &ChannelCredentials,
    ) -> Result<bool, ChannelError> {
        match credentials {
            ChannelCredentials::OAuth { access_token, .. } => {
                match self.get_user(access_token).await {
                    Ok(_) => Ok(true),
                    Err(ChannelError::AuthenticationFailed(_)) => Ok(false),
                    Err(e) => Err(e),
                }
            }
            _ => Ok(false),
        }
    }

    async fn refresh_token(&self, account: &mut ChannelAccount) -> Result<(), ChannelError> {
        let (refresh_token, client_id, client_secret) = match &account.credentials {
            ChannelCredentials::OAuth { refresh_token, .. } => {
                let refresh = refresh_token.as_ref().ok_or_else(|| {
                    ChannelError::AuthenticationFailed("No refresh token available".to_string())
                })?;
                let client_id = account
                    .settings
                    .custom
                    .get("client_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ChannelError::AuthenticationFailed("Missing client_id".to_string())
                    })?;
                let client_secret = account
                    .settings
                    .custom
                    .get("client_secret")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ChannelError::AuthenticationFailed("Missing client_secret".to_string())
                    })?;
                (
                    refresh.clone(),
                    client_id.to_string(),
                    client_secret.to_string(),
                )
            }
            _ => {
                return Err(ChannelError::AuthenticationFailed(
                    "OAuth credentials required".to_string(),
                ))
            }
        };

        let token_response = self
            .refresh_oauth_token(&client_id, &client_secret, &refresh_token)
            .await?;

        let expires_at =
            chrono::Utc::now() + chrono::Duration::seconds(token_response.expires_in as i64);

        account.credentials = ChannelCredentials::OAuth {
            access_token: token_response.access_token,
            refresh_token: Some(token_response.refresh_token),
            expires_at: Some(expires_at),
            scope: Some(token_response.scope),
        };

        Ok(())
    }
}

// ============================================================================
// Request Types
// ============================================================================
