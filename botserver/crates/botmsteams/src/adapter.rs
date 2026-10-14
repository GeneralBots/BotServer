use botlib::models::BotResponse;
use log::info;

use crate::state::{GetConfigFn, DbPool};
use crate::types::{
    TeamsAttachment, TeamsCardAction, TeamsCardImage, TeamsHeroCard, TeamsMember,
    TeamsOutboundActivity,
};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug)]
pub struct TeamsAdapter {
    app_id: String,
    app_password: String,
    tenant_id: String,
    service_url: String,
    bot_id: String,
}

impl TeamsAdapter {
    pub fn new(_pool: Arc<DbPool>, bot_id: Uuid, get_config: GetConfigFn) -> Self {
        let app_id = get_config(&bot_id, "teams-app-id", None).unwrap_or_default();
        let app_password = get_config(&bot_id, "teams-app-password", None).unwrap_or_default();
        let tenant_id = get_config(&bot_id, "teams-tenant-id", None).unwrap_or_default();

        let service_url = get_config(
            &bot_id,
            "teams-service-url",
            Some("https://smba.trafficmanager.net"),
        )
        .unwrap_or_else(|_| "https://smba.trafficmanager.net".to_string());

        let teams_bot_id = get_config(&bot_id, "teams-bot-id", None)
            .unwrap_or_else(|_| app_id.clone());

        Self {
            app_id,
            app_password,
            tenant_id,
            service_url,
            bot_id: teams_bot_id,
        }
    }

    pub async fn get_access_token(
        &self,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();

        let token_url = format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            if self.tenant_id.is_empty() {
                "botframework.com"
            } else {
                &self.tenant_id
            }
        );

        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", &self.app_id),
            ("client_secret", &self.app_password),
            ("scope", "https://api.botframework.com/.default"),
        ];

        let response = client.post(&token_url).form(&params).send().await?;

        if response.status().is_success() {
            let token_response: serde_json::Value = response.json().await?;
            Ok(token_response["access_token"]
                .as_str()
                .unwrap_or("")
                .to_string())
        } else {
            let error_text = response.text().await?;
            Err(format!("Failed to get access token: {}", error_text).into())
        }
    }

    async fn send_teams_message(
        &self,
        conversation_id: &str,
        activity: TeamsOutboundActivity,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();
        let token = self.get_access_token().await?;

        let url = format!(
            "{}/v3/conversations/{}/activities",
            self.service_url, conversation_id
        );

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&activity)
            .send()
            .await?;

        if response.status().is_success() {
            let result: serde_json::Value = response.json().await?;
            Ok(result["id"].as_str().unwrap_or("").to_string())
        } else {
            let error_text = response.text().await?;
            Err(format!("Teams API error: {}", error_text).into())
        }
    }

    pub async fn send_card(
        &self,
        conversation_id: &str,
        card: serde_json::Value,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let activity = TeamsOutboundActivity {
            activity_type: "message".to_string(),
            text: None,
            attachments: Some(vec![TeamsAttachment {
                content_type: "application/vnd.microsoft.card.adaptive".to_string(),
                content: card,
            }]),
            suggested_actions: None,
            channel_data: None,
        };

        self.send_teams_message(conversation_id, activity).await
    }

    pub async fn send_hero_card(
        &self,
        conversation_id: &str,
        title: &str,
        subtitle: Option<&str>,
        text: Option<&str>,
        images: Vec<String>,
        buttons: Vec<TeamsCardAction>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let hero_card = TeamsHeroCard {
            title: Some(title.to_string()),
            subtitle: subtitle.map(|s| s.to_string()),
            text: text.map(|s| s.to_string()),
            images: images
                .into_iter()
                .map(|url| TeamsCardImage { url, alt: None })
                .collect(),
            buttons: if buttons.is_empty() {
                None
            } else {
                Some(buttons)
            },
        };

        let activity = TeamsOutboundActivity {
            activity_type: "message".to_string(),
            text: None,
            attachments: Some(vec![TeamsAttachment {
                content_type: "application/vnd.microsoft.card.hero".to_string(),
                content: serde_json::to_value(hero_card)?,
            }]),
            suggested_actions: None,
            channel_data: None,
        };

        self.send_teams_message(conversation_id, activity).await
    }

    pub async fn create_conversation(
        &self,
        to: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();
        let token = self.get_access_token().await?;

        let url = format!("{}/v3/conversations", self.service_url);

        let payload = serde_json::json!({
            "bot": {
                "id": self.bot_id,
                "name": "Bot"
            },
            "members": [{"id": to}],
            "channelData": {
                "tenant": {"id": self.tenant_id}
            }
        });

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if response.status().is_success() {
            let result: serde_json::Value = response.json().await?;
            Ok(result["id"].as_str().unwrap_or("").to_string())
        } else {
            let error_text = response.text().await?;
            Err(format!("Failed to create conversation: {}", error_text).into())
        }
    }

    pub async fn update_message(
        &self,
        conversation_id: &str,
        activity_id: &str,
        new_text: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();
        let token = self.get_access_token().await?;

        let url = format!(
            "{}/v3/conversations/{}/activities/{}",
            self.service_url, conversation_id, activity_id
        );

        let activity = TeamsOutboundActivity {
            activity_type: "message".to_string(),
            text: Some(new_text.to_string()),
            attachments: None,
            suggested_actions: None,
            channel_data: None,
        };

        let response = client
            .put(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&activity)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Failed to update message: {}", error_text).into());
        }

        Ok(())
    }

    pub async fn delete_message(
        &self,
        conversation_id: &str,
        activity_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();
        let token = self.get_access_token().await?;

        let url = format!(
            "{}/v3/conversations/{}/activities/{}",
            self.service_url, conversation_id, activity_id
        );

        let response = client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Failed to delete message: {}", error_text).into());
        }

        Ok(())
    }

    pub async fn send_typing_indicator(
        &self,
        conversation_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let activity = TeamsOutboundActivity {
            activity_type: "typing".to_string(),
            text: None,
            attachments: None,
            suggested_actions: None,
            channel_data: None,
        };

        self.send_teams_message(conversation_id, activity).await?;
        Ok(())
    }

    pub async fn get_conversation_members(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<TeamsMember>, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();
        let token = self.get_access_token().await?;

        let url = format!(
            "{}/v3/conversations/{}/members",
            self.service_url, conversation_id
        );

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        if response.status().is_success() {
            let members: Vec<TeamsMember> = response.json().await?;
            Ok(members)
        } else {
            let error_text = response.text().await?;
            Err(format!("Failed to get conversation members: {}", error_text).into())
        }
    }
}

impl botlib::traits::ChannelAdapter for TeamsAdapter {
    fn channel_type(&self) -> &str {
        "msteams"
    }

    fn display_name(&self) -> &'static str {
        "Teams"
    }

    fn is_configured(&self) -> bool {
        !self.app_id.is_empty() && !self.app_password.is_empty()
    }

    fn send_bot_response<'a>(
        &'a self,
        response: BotResponse,
    ) -> botlib::traits::BoxFutureRefUnit<'a> {
        Box::pin(async move {
        if !self.is_configured() {
            log::error!("Teams adapter not configured. Please set teams-app-id and teams-app-password in the bot configuration database");
            return Err("Teams not configured".into());
        }

        let conversation_id = self.create_conversation(&response.user_id).await?;

        let activity = TeamsOutboundActivity {
            activity_type: "message".to_string(),
            text: Some(response.content.clone()),
            attachments: None,
            suggested_actions: None,
            channel_data: None,
        };

        let message_id = self.send_teams_message(&conversation_id, activity).await?;

        info!(
            "Teams message sent to conversation {}: (message_id: {})",
            conversation_id, message_id
        );

        Ok(())
        })
    }

    fn receive_message<'a>(
        &'a self,
        payload: serde_json::Value,
    ) -> botlib::traits::BoxFutureRefOptionString<'a> {
        let bot_id = self.bot_id.clone();
        Box::pin(async move {
        let activity_type = payload["type"].as_str().unwrap_or("");

        match activity_type {
            "message" => {
                if let Some(text) = payload["text"].as_str() {
                    let cleaned_text = text
                        .replace(&format!("<at>{}</at>", bot_id), "")
                        .trim()
                        .to_string();
                    Ok(Some(cleaned_text))
                } else if let Some(attachments) = payload["attachments"].as_array() {
                    if let Some(first_attachment) = attachments.first() {
                        let content_type = first_attachment["contentType"]
                            .as_str()
                            .unwrap_or("unknown");
                        Ok(Some(format!("Received attachment: {}", content_type)))
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            }
            "messageReaction" => {
                let reaction_type = payload["reactionsAdded"]
                    .as_array()
                    .and_then(|r| r.first())
                    .and_then(|r| r["type"].as_str())
                    .unwrap_or("unknown");
                Ok(Some(format!("Reaction: {}", reaction_type)))
            }
            _ => Ok(None),
        }
        })
    }

    fn get_user_info<'a>(
        &'a self,
        user_id: &str,
    ) -> botlib::traits::BoxFutureRefValue<'a> {
        let id = user_id.to_string();
        Box::pin(async move {
            Ok(serde_json::json!({
                "id": id,
                "platform": "teams"
            }))
        })
    }
}
