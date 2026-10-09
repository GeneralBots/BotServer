use super::*;

impl Default for YouTubeProvider {
    fn default() -> Self {
        Self::new()
    }
}
#[async_trait::async_trait]
impl ChannelProvider for YouTubeProvider {
    fn channel_type(&self) -> ChannelType {
        ChannelType::YouTube
    }

    fn max_text_length(&self) -> usize {
        5000 // Max description length for videos
    }

    fn supports_images(&self) -> bool {
        true // Thumbnails
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
                    "OAuth credentials required for YouTube".to_string(),
                ))
            }
        };

        let text = content.text.as_deref().unwrap_or("");

        // Get channel ID for community post
        let channel = self.get_channel(&access_token).await?;

        // Create community post with the content
        let post_request = CommunityPostRequest {
            channel_id: channel.id.clone(),
            text: text.to_string(),
            attached_video_id: content
                .metadata
                .get("video_id")
                .and_then(|v| v.as_str())
                .map(String::from),
            image_urls: content.image_urls.clone(),
        };

        let post = self.create_community_post(&access_token, &post_request).await?;

        let url = format!("https://www.youtube.com/post/{}", post.id);

        Ok(PostResult::success(ChannelType::YouTube, post.id, Some(url)))
    }

    async fn validate_credentials(
        &self,
        credentials: &ChannelCredentials,
    ) -> Result<bool, ChannelError> {
        match credentials {
            ChannelCredentials::OAuth { access_token, .. } => {
                match self.get_channel(access_token).await {
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
                (refresh.clone(), client_id.to_string(), client_secret.to_string())
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

        let expires_at = chrono::Utc::now()
            + chrono::Duration::seconds(token_response.expires_in.unwrap_or(3600) as i64);

        account.credentials = ChannelCredentials::OAuth {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token.or(Some(refresh_token)),
            expires_at: Some(expires_at),
            scope: token_response.scope,
        };

        Ok(())
    }
}
