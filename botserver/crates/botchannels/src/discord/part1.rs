use super::*;

impl DiscordProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://discord.com/api/v10".to_string(),
        }
    }

    async fn send_message(
        &self,
        bot_token: &str,
        channel_id: &str,
        content: &MessageContent,
    ) -> Result<DiscordMessage, ChannelError> {
        let response = self
            .client
            .post(format!("{}/channels/{}/messages", self.base_url, channel_id))
            .header("Authorization", format!("Bot {}", bot_token))
            .header("Content-Type", "application/json")
            .json(content)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        let status = response.status();

        if status.as_u16() == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok());
            return Err(ChannelError::RateLimited { retry_after });
        }

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ChannelError::ApiError {
                code: Some(status.to_string()),
                message: error_text,
            });
        }

        response
            .json::<DiscordMessage>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }

    async fn send_webhook_message(
        &self,
        webhook_url: &str,
        content: &WebhookContent,
    ) -> Result<DiscordMessage, ChannelError> {
        let url = format!("{}?wait=true", webhook_url);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(content)
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        let status = response.status();

        if status.as_u16() == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok());
            return Err(ChannelError::RateLimited { retry_after });
        }

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ChannelError::ApiError {
                code: Some(status.to_string()),
                message: error_text,
            });
        }

        response
            .json::<DiscordMessage>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }

    async fn get_current_user(&self, bot_token: &str) -> Result<DiscordUser, ChannelError> {
        let response = self
            .client
            .get(format!("{}/users/@me", self.base_url))
            .header("Authorization", format!("Bot {}", bot_token))
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ChannelError::AuthenticationFailed(error_text));
        }

        response
            .json::<DiscordUser>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }

    pub(super) fn build_message_url(guild_id: &str, channel_id: &str, message_id: &str) -> String {
        format!(
            "https://discord.com/channels/{}/{}/{}",
            guild_id, channel_id, message_id
        )
    }

    pub(super) fn create_embeds(content: &PostContent) -> Vec<DiscordEmbed> {
        let mut embeds = Vec::new();

        if content.link.is_some() || !content.image_urls.is_empty() {
            let mut embed = DiscordEmbed {
                title: None,
                description: None,
                url: content.link.clone(),
                color: Some(0x5865F2),
                image: None,
                thumbnail: None,
                fields: vec![],
                footer: None,
                timestamp: None,
            };

            if let Some(first_image) = content.image_urls.first() {
                embed.image = Some(EmbedImage {
                    url: first_image.clone(),
                });
            }

            embeds.push(embed);

            for image_url in content.image_urls.iter().skip(1).take(3) {
                embeds.push(DiscordEmbed {
                    title: None,
                    description: None,
                    url: None,
                    color: None,
                    image: Some(EmbedImage {
                        url: image_url.clone(),
                    }),
                    thumbnail: None,
                    fields: vec![],
                    footer: None,
                    timestamp: None,
                });
            }
        }

        embeds
    }
}
impl Default for DiscordProvider {
    fn default() -> Self {
        Self::new()
    }
}
