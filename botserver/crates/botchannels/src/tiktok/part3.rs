use super::*;

impl Default for TikTokProvider {
    fn default() -> Self {
        Self::new()
    }
}
#[async_trait::async_trait]
impl ChannelProvider for TikTokProvider {
    fn channel_type(&self) -> ChannelType {
        ChannelType::TikTok
    }

    fn max_text_length(&self) -> usize {
        2200 // Max caption length
    }

    fn supports_images(&self) -> bool {
        true // Photo mode
    }

    fn supports_video(&self) -> bool {
        true
    }

    fn supports_links(&self) -> bool {
        false // Links not clickable in TikTok captions
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
                    "OAuth credentials required for TikTok".to_string(),
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

        // Determine privacy level from settings or default
        let privacy_level = account
            .settings
            .custom
            .get("default_privacy")
            .and_then(|v| v.as_str())
            .unwrap_or("SELF_ONLY")
            .to_string();

        // Check if we're posting a video or photos
        if let Some(video_url) = &content.video_url {
            // Video post via URL
            let request = VideoUploadRequest {
                title: text.to_string(),
                privacy_level,
                video_url: Some(video_url.clone()),
                disable_duet: Some(false),
                disable_comment: Some(false),
                disable_stitch: Some(false),
                video_cover_timestamp_ms: None,
                brand_content_toggle: None,
                brand_organic_toggle: None,
                video_size: None,
                chunk_size: None,
                total_chunk_count: None,
            };

            let init_result = self.init_video_upload(&access_token, &request).await?;

            Ok(PostResult::success(
                ChannelType::TikTok,
                init_result.publish_id,
                None, // URL not available until processing complete
            ))
        } else if !content.image_urls.is_empty() {
            // Photo post
            let request = PhotoPostRequest {
                title: text.to_string(),
                description: content
                    .metadata
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                privacy_level,
                photo_urls: content.image_urls.clone(),
                photo_cover_index: Some(0),
                disable_comment: Some(false),
                auto_add_music: Some(true),
            };

            let init_result = self.init_photo_post(&access_token, &request).await?;

            Ok(PostResult::success(
                ChannelType::TikTok,
                init_result.publish_id,
                None,
            ))
        } else {
            Err(ChannelError::ApiError {
                code: None,
                message: "TikTok requires either a video or photos to post".to_string(),
            })
        }
    }

    async fn validate_credentials(
        &self,
        credentials: &ChannelCredentials,
    ) -> Result<bool, ChannelError> {
        match credentials {
            ChannelCredentials::OAuth { access_token, .. } => {
                match self
                    .get_user_info(access_token, &["open_id", "display_name"])
                    .await
                {
                    Ok(_) => Ok(true),
                    Err(ChannelError::AuthenticationFailed(_)) => Ok(false),
                    Err(e) => Err(e),
                }
            }
            _ => Ok(false),
        }
    }

    async fn refresh_token(&self, account: &mut ChannelAccount) -> Result<(), ChannelError> {
        let (refresh_token, client_key, client_secret) = match &account.credentials {
            ChannelCredentials::OAuth { refresh_token, .. } => {
                let refresh = refresh_token.as_ref().ok_or_else(|| {
                    ChannelError::AuthenticationFailed("No refresh token available".to_string())
                })?;
                let client_key = account
                    .settings
                    .custom
                    .get("client_key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ChannelError::AuthenticationFailed("Missing client_key".to_string())
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
                    client_key.to_string(),
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
            .refresh_oauth_token(&client_key, &client_secret, &refresh_token)
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
