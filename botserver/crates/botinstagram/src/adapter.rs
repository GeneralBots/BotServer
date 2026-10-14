use botlib::models::BotResponse;
use log::info;

use crate::state::GetConfigFn;
use crate::types::InstagramProfile;

#[derive(Debug)]
pub struct InstagramAdapter {
    access_token: String,
    verify_token: String,
    page_id: String,
    instagram_account_id: String,
}

impl Default for InstagramAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl InstagramAdapter {
    /// Graph API version is a compile-time constant; the access token travels
    /// in the query string and must only ever reach graph.facebook.com.
    const API_VERSION: &str = "v17.0";

    pub fn new() -> Self {
        let access_token = String::new();
        let verify_token = "webhook_verify".to_string();
        let page_id = String::new();
        let instagram_account_id = String::new();

        Self {
            access_token,
            verify_token,
            page_id,
            instagram_account_id,
        }
    }

    pub fn with_config(get_config: &GetConfigFn, bot_id: &str) -> Self {
        let access_token = get_config(bot_id, "instagram-access-token", None).unwrap_or_default();
        let verify_token = get_config(bot_id, "instagram-verify-token", Some("webhook_verify"))
            .unwrap_or_else(|_| "webhook_verify".to_string());
        let page_id = get_config(bot_id, "instagram-page-id", None).unwrap_or_default();
        let instagram_account_id =
            get_config(bot_id, "instagram-account-id", None).unwrap_or_default();

        Self {
            access_token,
            verify_token,
            page_id,
            instagram_account_id,
        }
    }

    pub fn get_instagram_account_id(&self) -> &str {
        &self.instagram_account_id
    }

    pub async fn get_instagram_business_account(
        &self,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();

        let url = format!(
            "https://graph.facebook.com/{}/{}/instagram_business_account",
            Self::API_VERSION, self.page_id
        );

        let response = client
            .get(&url)
            .query(&[("access_token", &self.access_token)])
            .send()
            .await?;

        if response.status().is_success() {
            let result: serde_json::Value = response.json().await?;
            Ok(result["id"]
                .as_str()
                .unwrap_or(&self.instagram_account_id)
                .to_string())
        } else {
            Ok(self.instagram_account_id.clone())
        }
    }

    pub async fn post_to_instagram(
        &self,
        image_url: &str,
        caption: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();
        let account_id = if self.instagram_account_id.is_empty() {
            self.get_instagram_business_account().await?
        } else {
            self.instagram_account_id.clone()
        };

        let container_url = format!(
            "https://graph.facebook.com/{}/{}/media",
            Self::API_VERSION, account_id
        );

        let container_response = client
            .post(&container_url)
            .query(&[
                ("access_token", &self.access_token),
                ("image_url", &image_url.to_string()),
                ("caption", &caption.to_string()),
            ])
            .send()
            .await?;

        if !container_response.status().is_success() {
            let error_text = container_response.text().await?;
            return Err(format!("Failed to create media container: {}", error_text).into());
        }

        let container_result: serde_json::Value = container_response.json().await?;
        let creation_id = container_result["id"]
            .as_str()
            .ok_or("No creation_id in response")?;

        let publish_url = format!(
            "https://graph.facebook.com/{}/{}/media_publish",
            Self::API_VERSION, account_id
        );

        let publish_response = client
            .post(&publish_url)
            .query(&[
                ("access_token", &self.access_token),
                ("creation_id", &creation_id.to_string()),
            ])
            .send()
            .await?;

        if publish_response.status().is_success() {
            let publish_result: serde_json::Value = publish_response.json().await?;
            Ok(publish_result["id"].as_str().unwrap_or("").to_string())
        } else {
            let error_text = publish_response.text().await?;
            Err(format!("Failed to publish media: {}", error_text).into())
        }
    }

    pub async fn send_instagram_message(
        &self,
        recipient_id: &str,
        message: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();

        let url = format!(
            "https://graph.facebook.com/{}/{}/messages",
            Self::API_VERSION, self.page_id
        );

        let payload = serde_json::json!({
            "recipient": {"id": recipient_id},
            "message": {"text": message},
            "messaging_type": "RESPONSE"
        });

        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .query(&[("access_token", &self.access_token)])
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            let result: serde_json::Value = response.json().await?;
            Ok(result["message_id"].as_str().unwrap_or("").to_string())
        } else {
            let error_text = response.text().await?;
            Err(format!("Instagram API error: {}", error_text).into())
        }
    }

    pub async fn send_media_message(
        &self,
        recipient_id: &str,
        media_url: &str,
        media_type: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();

        let url = format!(
            "https://graph.facebook.com/{}/{}/messages",
            Self::API_VERSION, self.page_id
        );

        let attachment_type = match media_type {
            "image" => "image",
            "video" => "video",
            "audio" => "audio",
            _ => "file",
        };

        let payload = serde_json::json!({
            "recipient": {"id": recipient_id},
            "message": {
                "attachment": {
                    "type": attachment_type,
                    "payload": {"url": media_url, "is_reusable": true}
                }
            }
        });

        let response = client
            .post(&url)
            .query(&[("access_token", &self.access_token)])
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            let result: serde_json::Value = response.json().await?;
            Ok(result["message_id"].as_str().unwrap_or("").to_string())
        } else {
            let error_text = response.text().await?;
            Err(format!("Instagram API error: {}", error_text).into())
        }
    }

    pub async fn send_story_reply(
        &self,
        recipient_id: &str,
        message: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        self.send_instagram_message(recipient_id, message).await
    }

    pub async fn get_user_profile(
        &self,
        user_id: &str,
    ) -> Result<InstagramProfile, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();

        let url = format!(
            "https://graph.facebook.com/{}/{}",
            Self::API_VERSION, user_id
        );

        let response = client
            .get(&url)
            .query(&[
                ("access_token", &self.access_token),
                ("fields", &"name,profile_pic".to_string()),
            ])
            .send()
            .await?;

        if response.status().is_success() {
            let profile: InstagramProfile = response.json().await?;
            Ok(profile)
        } else {
            Err("Failed to get Instagram profile".into())
        }
    }

    pub fn verify_webhook(&self, token: &str) -> bool {
        token == self.verify_token
    }

    pub fn handle_webhook_verification(
        &self,
        mode: &str,
        token: &str,
        challenge: &str,
    ) -> Option<String> {
        if mode == "subscribe" && self.verify_webhook(token) {
            Some(challenge.to_string())
        } else {
            None
        }
    }
}

impl botlib::traits::ChannelAdapter for InstagramAdapter {
    fn channel_type(&self) -> &str {
        "instagram"
    }

    fn display_name(&self) -> &'static str {
        "Instagram"
    }

    fn is_configured(&self) -> bool {
        !self.access_token.is_empty() && !self.page_id.is_empty()
    }

    fn send_bot_response<'a>(
        &'a self,
        response: BotResponse,
    ) -> botlib::traits::BoxFutureRefUnit<'a> {
        Box::pin(async move {
        if !self.is_configured() {
            log::error!("Instagram adapter not configured. Please set instagram-access-token and instagram-page-id in the bot configuration database");
            return Err("Instagram not configured".into());
        }

        let message_id = self
            .send_instagram_message(&response.user_id, &response.content)
            .await?;

        info!(
            "Instagram message sent to {}: {} (message_id: {})",
            response.user_id, response.content, message_id
        );

        Ok(())
        })
    }

    fn receive_message<'a>(
        &'a self,
        payload: serde_json::Value,
    ) -> botlib::traits::BoxFutureRefOptionString<'a> {
        Box::pin(async move {
        if let Some(entry) = payload["entry"].as_array() {
            if let Some(first_entry) = entry.first() {
                if let Some(messaging) = first_entry["messaging"].as_array() {
                    if let Some(first_message) = messaging.first() {
                        if let Some(message) = first_message["message"].as_object() {
                            if let Some(text) = message["text"].as_str() {
                                return Ok(Some(text.to_string()));
                            } else if let Some(attachments) = message["attachments"].as_array() {
                                if let Some(first_attachment) = attachments.first() {
                                    let attachment_type =
                                        first_attachment["type"].as_str().unwrap_or("unknown");
                                    return Ok(Some(format!(
                                        "Received {} attachment",
                                        attachment_type
                                    )));
                                }
                            }
                        } else if let Some(postback) = first_message["postback"].as_object() {
                            if let Some(payload_str) = postback["payload"].as_str() {
                                return Ok(Some(format!("Postback: {}", payload_str)));
                            }
                        }
                    }
                } else if let Some(changes) = first_entry["changes"].as_array() {
                    if let Some(first_change) = changes.first() {
                        let field = first_change["field"].as_str().unwrap_or("");
                        match field {
                            "comments" => {
                                if let Some(text) = first_change["value"]["text"].as_str() {
                                    return Ok(Some(format!("Comment: {}", text)));
                                }
                            }
                            "mentions" => {
                                if let Some(media_id) =
                                    first_change["value"]["media_id"].as_str()
                                {
                                    return Ok(Some(format!("Mentioned in media: {}", media_id)));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(None)
        })
    }

    fn get_user_info<'a>(
        &'a self,
        user_id: &str,
    ) -> botlib::traits::BoxFutureRefValue<'a> {
        let id = user_id.to_string();
        Box::pin(async move {
            match self.get_user_profile(&id).await {
                Ok(profile) => Ok(serde_json::to_value(profile)
                    .map_err(|e| format!("profile serialization failed: {e}"))?),
                Err(_) => Ok(serde_json::json!({
                    "id": id,
                    "platform": "instagram"
                })),
            }
        })
    }
}
