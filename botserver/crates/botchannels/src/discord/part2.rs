use super::*;

#[async_trait::async_trait]
impl ChannelProvider for DiscordProvider {
    fn channel_type(&self) -> ChannelType {
        ChannelType::Discord
    }

    fn max_text_length(&self) -> usize {
        2000
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
        let text = content.text.as_deref().unwrap_or("");

        if text.len() > self.max_text_length() {
            return Err(ChannelError::ContentTooLong {
                max_length: self.max_text_length(),
                actual_length: text.len(),
            });
        }

        let mut full_text = text.to_string();

        if !content.hashtags.is_empty() {
            let tags = content
                .hashtags
                .iter()
                .map(|t| {
                    if t.starts_with('#') {
                        t.clone()
                    } else {
                        format!("#{}", t)
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            full_text = format!("{}\n\n{}", full_text, tags);
        }

        match &account.credentials {
            ChannelCredentials::ApiKey { api_key, api_secret } => {
                let channel_id = api_secret
                    .as_ref()
                    .ok_or_else(|| ChannelError::AuthenticationFailed(
                        "Channel ID required in api_secret".to_string(),
                    ))?;

                let embeds = Self::create_embeds(content);

                let message_content = MessageContent {
                    content: if full_text.is_empty() { None } else { Some(full_text) },
                    embeds: if embeds.is_empty() { None } else { Some(embeds) },
                    tts: Some(false),
                };

                let message = self.send_message(api_key, channel_id, &message_content).await?;

                let guild_id = account
                    .settings
                    .custom
                    .get("guild_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("@me");

                let url = Self::build_message_url(guild_id, channel_id, &message.id);

                Ok(PostResult::success(
                    ChannelType::Discord,
                    message.id,
                    Some(url),
                ))
            }
            ChannelCredentials::Custom { data } => {
                let webhook_url = data
                    .get("webhook_url")
                    .ok_or_else(|| ChannelError::AuthenticationFailed(
                        "Webhook URL required".to_string(),
                    ))?;

                let username = data.get("username").cloned();
                let avatar_url = data.get("avatar_url").cloned();

                let embeds = Self::create_embeds(content);

                let webhook_content = WebhookContent {
                    content: if full_text.is_empty() { None } else { Some(full_text) },
                    username,
                    avatar_url,
                    embeds: if embeds.is_empty() { None } else { Some(embeds) },
                    tts: Some(false),
                };

                let message = self.send_webhook_message(webhook_url, &webhook_content).await?;

                Ok(PostResult::success(
                    ChannelType::Discord,
                    message.id,
                    None,
                ))
            }
            _ => Err(ChannelError::AuthenticationFailed(
                "Invalid credentials type for Discord".to_string(),
            )),
        }
    }

    async fn validate_credentials(
        &self,
        credentials: &ChannelCredentials,
    ) -> Result<bool, ChannelError> {
        match credentials {
            ChannelCredentials::ApiKey { api_key, .. } => {
                match self.get_current_user(api_key).await {
                    Ok(_) => Ok(true),
                    Err(ChannelError::AuthenticationFailed(_)) => Ok(false),
                    Err(e) => Err(e),
                }
            }
            ChannelCredentials::Custom { data } => {
                let webhook_url = data.get("webhook_url");
                Ok(webhook_url.is_some())
            }
            _ => Ok(false),
        }
    }

    async fn refresh_token(&self, _account: &mut ChannelAccount) -> Result<(), ChannelError> {
        Ok(())
    }
}
