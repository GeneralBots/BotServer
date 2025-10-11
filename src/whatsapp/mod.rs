use async_trait::async_trait;
use log::info;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::shared::models::BotResponse;

#[derive(Debug, Deserialize)]
pub struct WhatsAppMessage {
    pub entry: Vec<WhatsAppEntry>,
}

#[derive(Debug, Deserialize)]
pub struct WhatsAppEntry {
    pub changes: Vec<WhatsAppChange>,
}

#[derive(Debug, Deserialize)]
pub struct WhatsAppChange {
    pub value: WhatsAppValue,
}

#[derive(Debug, Deserialize)]
pub struct WhatsAppValue {
    pub contacts: Option<Vec<WhatsAppContact>>,
    pub messages: Option<Vec<WhatsAppMessageData>>,
}

#[derive(Debug, Deserialize)]
pub struct WhatsAppContact {
    pub profile: WhatsAppProfile,
    pub wa_id: String,
}

#[derive(Debug, Deserialize)]
pub struct WhatsAppProfile {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct WhatsAppMessageData {
    pub from: String,
    pub id: String,
    pub timestamp: String,
    pub text: Option<WhatsAppText>,
    pub r#type: String,
}

#[derive(Debug, Deserialize)]
pub struct WhatsAppText {
    pub body: String,
}

#[derive(Serialize)]
pub struct WhatsAppResponse {
    pub messaging_product: String,
    pub to: String,
    pub text: WhatsAppResponseText,
}

#[derive(Serialize)]
pub struct WhatsAppResponseText {
    pub body: String,
}

pub struct WhatsAppAdapter {
    client: Client,
    access_token: String,
    phone_number_id: String,
    webhook_verify_token: String,
    sessions: Arc<Mutex<HashMap<String, String>>>,
}

impl WhatsAppAdapter {
    pub fn new(
        access_token: String,
        phone_number_id: String,
        webhook_verify_token: String,
    ) -> Self {
        Self {
            client: Client::new(),
            access_token,
            phone_number_id,
            webhook_verify_token,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn get_session_id(&self, phone: &str) -> String {
        let sessions = self.sessions.lock().await;
        if let Some(session_id) = sessions.get(phone) {
            session_id.clone()
        } else {
            drop(sessions);
            let session_id = uuid::Uuid::new_v4().to_string();
            let mut sessions = self.sessions.lock().await;
            sessions.insert(phone.to_string(), session_id.clone());
            session_id
        }
    }

    pub async fn send_whatsapp_message(
        &self,
        to: &str,
        body: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = format!(
            "https://graph.facebook.com/v17.0/{}/messages",
            self.phone_number_id
        );

        let response_data = WhatsAppResponse {
            messaging_product: "whatsapp".to_string(),
            to: to.to_string(),
            text: WhatsAppResponseText {
                body: body.to_string(),
            },
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(&response_data)
            .send()
            .await?;

        if response.status().is_success() {
            info!("WhatsApp message sent to {}", to);
        } else {
            let error_text = response.text().await?;
            log::error!("Failed to send WhatsApp message: {}", error_text);
        }

        Ok(())
    }

    pub async fn process_incoming_message(
        &self,
        message: WhatsAppMessage,
    ) -> Result<Vec<crate::shared::UserMessage>, Box<dyn std::error::Error + Send + Sync>> {
        let mut user_messages = Vec::new();

        for entry in message.entry {
            for change in entry.changes {
                if let Some(messages) = change.value.messages {
                    for msg in messages {
                        if let Some(text) = msg.text {
                            let session_id = self.get_session_id(&msg.from).await;

                            let user_message = crate::shared::models::UserMessage {
                                bot_id: "default_bot".to_string(),
                                user_id: msg.from.clone(),
                                session_id: session_id.clone(),
                                channel: "whatsapp".to_string(),
                                content: text.body,
                                message_type: msg.r#type,
                                media_url: None,
                                timestamp: chrono::Utc::now(),
                            };

                            user_messages.push(user_message);
                        }
                    }
                }
            }
        }

        Ok(user_messages)
    }

    pub fn verify_webhook(
        &self,
        mode: &str,
        token: &str,
        challenge: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if mode == "subscribe" && token == self.webhook_verify_token {
            Ok(challenge.to_string())
        } else {
            Err("Invalid verification".into())
        }
    }
}

#[async_trait]
impl crate::channels::ChannelAdapter for WhatsAppAdapter {
    async fn send_message(
        &self,
        response: BotResponse,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Sending WhatsApp response to: {}", response.user_id);
        self.send_whatsapp_message(&response.user_id, &response.content)
            .await
    }
}
