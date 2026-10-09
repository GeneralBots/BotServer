use super::*;

impl DiscordProvider {
    pub async fn send_dm(
        &self,
        bot_token: &str,
        user_id: &str,
        message: &str,
    ) -> Result<DiscordMessage, ChannelError> {
        let dm_channel = self
            .client
            .post(format!("{}/users/@me/channels", self.base_url))
            .header("Authorization", format!("Bot {}", bot_token))
            .json(&CreateDMChannel {
                recipient_id: user_id.to_string(),
            })
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !dm_channel.status().is_success() {
            let error_text = dm_channel.text().await.unwrap_or_default();
            return Err(ChannelError::ApiError {
                code: None,
                message: format!("Failed to create DM channel: {}", error_text),
            });
        }

        #[derive(Deserialize)]
        struct DMChannel {
            id: String,
        }

        let channel: DMChannel = dm_channel
            .json()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })?;

        let content = MessageContent {
            content: Some(message.to_string()),
            embeds: None,
            tts: Some(false),
        };

        self.send_message(bot_token, &channel.id, &content).await
    }

    pub async fn get_guild_channels(
        &self,
        bot_token: &str,
        guild_id: &str,
    ) -> Result<Vec<GuildChannel>, ChannelError> {
        let response = self
            .client
            .get(format!("{}/guilds/{}/channels", self.base_url, guild_id))
            .header("Authorization", format!("Bot {}", bot_token))
            .send()
            .await
            .map_err(|e| ChannelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ChannelError::ApiError {
                code: None,
                message: error_text,
            });
        }

        response
            .json::<Vec<GuildChannel>>()
            .await
            .map_err(|e| ChannelError::ApiError {
                code: None,
                message: e.to_string(),
            })
    }
}
