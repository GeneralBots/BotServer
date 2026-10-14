use botlib::models::BotResponse;
use log::{debug, info};
use serde::{Deserialize, Serialize};

use crate::state::{GetConfigFn, DbPool};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct TelegramSendMessage {
    chat_id: String,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parse_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_markup: Option<TelegramReplyMarkup>,
}

#[derive(Debug, Serialize)]
struct TelegramReplyMarkup {
    #[serde(skip_serializing_if = "Option::is_none")]
    inline_keyboard: Option<Vec<Vec<TelegramInlineButton>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    keyboard: Option<Vec<Vec<TelegramKeyboardButton>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    one_time_keyboard: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resize_keyboard: Option<bool>,
}

#[derive(Debug, Serialize)]
struct TelegramInlineButton {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    callback_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
}

#[derive(Debug, Serialize)]
struct TelegramKeyboardButton {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_contact: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_location: Option<bool>,
}

#[derive(Debug, Serialize)]
struct TelegramSendPhoto {
    chat_id: String,
    photo: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    caption: Option<String>,
}

#[derive(Debug, Serialize)]
struct TelegramSendDocument {
    chat_id: String,
    document: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    caption: Option<String>,
}

#[derive(Debug, Serialize)]
struct TelegramSendLocation {
    chat_id: String,
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Deserialize)]
pub struct TelegramResponse {
    pub ok: bool,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug)]
pub struct TelegramAdapter {
    bot_token: String,
}

impl TelegramAdapter {
    pub fn new(_pool: Arc<DbPool>, bot_id: Uuid, get_config: GetConfigFn) -> Self {
        let bot_token = get_config(&bot_id, "telegram-bot-token", None).unwrap_or_default();

        Self { bot_token }
    }

    async fn send_telegram_request<T: Serialize>(
        &self,
        method: &str,
        payload: &T,
    ) -> Result<TelegramResponse, Box<dyn std::error::Error + Send + Sync>> {
        if self.bot_token.is_empty() {
            return Err("Telegram bot token not configured".into());
        }

        let url = format!("https://api.telegram.org/bot{}/{}", self.bot_token, method);

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .json(payload)
            .send()
            .await?
            .json::<TelegramResponse>()
            .await?;

        if !response.ok {
            let error_msg = response
                .description
                .unwrap_or_else(|| "Unknown Telegram API error".to_string());
            log::error!("Telegram API error: {}", error_msg);
            return Err(error_msg.into());
        }

        Ok(response)
    }

    pub async fn send_text_message(
        &self,
        chat_id: &str,
        text: &str,
        parse_mode: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = TelegramSendMessage {
            chat_id: chat_id.to_string(),
            text: text.to_string(),
            parse_mode: parse_mode.map(String::from),
            reply_markup: None,
        };

        self.send_telegram_request("sendMessage", &payload).await?;
        info!("Telegram message sent to chat {}", chat_id);
        Ok(())
    }

    pub async fn send_message_with_buttons(
        &self,
        chat_id: &str,
        text: &str,
        buttons: Vec<(String, String)>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let inline_buttons: Vec<Vec<TelegramInlineButton>> = buttons
            .into_iter()
            .map(|(label, callback)| {
                vec![TelegramInlineButton {
                    text: label,
                    callback_data: Some(callback),
                    url: None,
                }]
            })
            .collect();

        let payload = TelegramSendMessage {
            chat_id: chat_id.to_string(),
            text: text.to_string(),
            parse_mode: Some("HTML".to_string()),
            reply_markup: Some(TelegramReplyMarkup {
                inline_keyboard: Some(inline_buttons),
                keyboard: None,
                one_time_keyboard: None,
                resize_keyboard: None,
            }),
        };

        self.send_telegram_request("sendMessage", &payload).await?;
        info!("Telegram message with buttons sent to chat {}", chat_id);
        Ok(())
    }

    pub async fn send_photo(
        &self,
        chat_id: &str,
        photo_url: &str,
        caption: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = TelegramSendPhoto {
            chat_id: chat_id.to_string(),
            photo: photo_url.to_string(),
            caption: caption.map(String::from),
        };

        self.send_telegram_request("sendPhoto", &payload).await?;
        info!("Telegram photo sent to chat {}", chat_id);
        Ok(())
    }

    pub async fn send_document(
        &self,
        chat_id: &str,
        document_url: &str,
        caption: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = TelegramSendDocument {
            chat_id: chat_id.to_string(),
            document: document_url.to_string(),
            caption: caption.map(String::from),
        };

        self.send_telegram_request("sendDocument", &payload).await?;
        info!("Telegram document sent to chat {}", chat_id);
        Ok(())
    }

    pub async fn send_location(
        &self,
        chat_id: &str,
        latitude: f64,
        longitude: f64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = TelegramSendLocation {
            chat_id: chat_id.to_string(),
            latitude,
            longitude,
        };

        self.send_telegram_request("sendLocation", &payload).await?;
        info!("Telegram location sent to chat {}", chat_id);
        Ok(())
    }

    /// Resolves a Telegram `file_id` into the temporary path that
    /// `download_file` can serve (`POST getFile`).
    pub async fn get_file(
        &self,
        file_id: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        #[derive(Serialize)]
        struct GetFile<'a> {
            file_id: &'a str,
        }

        let response = self
            .send_telegram_request("getFile", &GetFile { file_id })
            .await?;

        response
            .result
            .as_ref()
            .and_then(|result| result.get("file_path"))
            .and_then(|path| path.as_str())
            .map(|path| path.to_string())
            .ok_or_else(|| "Telegram getFile returned no file_path".into())
    }

    /// Downloads a file previously resolved by `get_file`. Telegram caps
    /// `getFile` at 20 MB per file.
    pub async fn download_file(
        &self,
        file_path: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        if self.bot_token.is_empty() {
            return Err("Telegram bot token not configured".into());
        }

        let url = format!(
            "https://api.telegram.org/file/bot{}/{}",
            self.bot_token, file_path
        );

        let response = reqwest::Client::new().get(&url).send().await?;

        if !response.status().is_success() {
            return Err(format!(
                "Telegram file download failed with status {}",
                response.status()
            )
            .into());
        }

        Ok(response.bytes().await?.to_vec())
    }

    pub async fn set_webhook(
        &self,
        webhook_url: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        #[derive(Serialize)]
        struct SetWebhook {
            url: String,
            allowed_updates: Vec<String>,
        }

        let payload = SetWebhook {
            url: webhook_url.to_string(),
            allowed_updates: vec![
                "message".to_string(),
                "callback_query".to_string(),
                "edited_message".to_string(),
            ],
        };

        self.send_telegram_request("setWebhook", &payload).await?;
        info!("Telegram webhook set to {}", webhook_url);
        Ok(())
    }

    pub async fn delete_webhook(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        #[derive(Serialize)]
        struct DeleteWebhook {
            drop_pending_updates: bool,
        }

        let payload = DeleteWebhook {
            drop_pending_updates: false,
        };

        self.send_telegram_request("deleteWebhook", &payload).await?;
        info!("Telegram webhook deleted");
        Ok(())
    }

    pub async fn get_me(&self) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        #[derive(Serialize)]
        struct Empty {}

        let response = self.send_telegram_request("getMe", &Empty {}).await?;
        Ok(response.result.unwrap_or(serde_json::Value::Null))
    }
}

impl botlib::traits::ChannelAdapter for TelegramAdapter {
    fn channel_type(&self) -> &str {
        "telegram"
    }

    fn display_name(&self) -> &'static str {
        "Telegram"
    }

    fn is_configured(&self) -> bool {
        !self.bot_token.is_empty()
    }

    fn send_bot_response<'a>(
        &'a self,
        response: BotResponse,
    ) -> botlib::traits::BoxFutureRefUnit<'a> {
        Box::pin(async move {
            if !self.is_configured() {
                log::error!("Telegram adapter not configured. Please write telegram-bot-token to the bot's Vault path (secret/gbo/{{org_id}}/{{branch_id}}/{{bot_id}})");
                return Err("Telegram not configured".into());
            }

            let chat_id = &response.user_id;

            // LLM output is not guaranteed to be valid HTML — Telegram rejects
            // the whole message on any parse-entity error ("can't parse
            // entities: Unsupported start tag \"?xml\""). Sanitize up front so
            // supported tags survive and anything else is neutralized; the
            // plain-text retry below stays as a last-resort safety net.
            let safe_html = crate::html::sanitize_for_html(&response.content);
            if let Err(e) = self.send_text_message(chat_id, &safe_html, Some("HTML")).await {
                log::warn!("Telegram HTML send failed ({e}); retrying as plain text");
                self.send_text_message(chat_id, &response.content, None).await?;
            }

            debug!(
                "Telegram message sent to {} for session {}",
                chat_id, response.session_id
            );
            Ok(())
        })
    }

    fn get_user_info<'a>(
        &'a self,
        user_id: &str,
    ) -> botlib::traits::BoxFutureRefValue<'a> {
        let chat_id = user_id.to_string();
        Box::pin(async move {
            Ok(serde_json::json!({
                "id": chat_id,
                "platform": "telegram",
                "chat_id": chat_id
            }))
        })
    }
}
