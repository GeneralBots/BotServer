use super::{new_expectation_store, ExpectationStore};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub struct MockTeams {
    server: MockServer,
    port: u16,
    expectations: ExpectationStore,
    sent_activities: Arc<Mutex<Vec<Activity>>>,
    conversations: Arc<Mutex<HashMap<String, ConversationInfo>>>,
    bot_id: String,
    bot_name: String,
    tenant_id: String,
    service_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    #[serde(rename = "type")]
    pub kind: String,
    pub id: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_timestamp: Option<String>,
    pub service_url: String,
    pub channel_id: String,
    pub from: ChannelAccount,
    pub conversation: ConversationAccount,
    pub recipient: ChannelAccount,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<Attachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entities: Option<Vec<Entity>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Default for Activity {
    fn default() -> Self {
        Self {
            kind: "message".to_string(),
            id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            local_timestamp: None,
            service_url: String::new(),
            channel_id: "msteams".to_string(),
            from: ChannelAccount::default(),
            conversation: ConversationAccount::default(),
            recipient: ChannelAccount::default(),
            text: None,
            text_format: Some("plain".to_string()),
            locale: Some("en-US".to_string()),
            attachments: None,
            entities: None,
            channel_data: None,
            action: None,
            reply_to_id: None,
            value: None,
            name: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChannelAccount {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aad_object_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConversationAccount {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_group: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mentioned: Option<ChannelAccount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct ConversationInfo {
    pub id: String,
    pub tenant_id: String,
    pub service_url: String,
    pub members: Vec<ChannelAccount>,
    pub is_group: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceResponse {
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationsResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation_token: Option<String>,
    pub conversations: Vec<ConversationMembers>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationMembers {
    pub id: String,
    pub members: Vec<ChannelAccount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamsChannelAccount {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aad_object_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_principal_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub given_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surname: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

impl MockTeams {
    pub const DEFAULT_BOT_ID: &'static str = "28:test-bot-id";

    pub const DEFAULT_BOT_NAME: &'static str = "TestBot";

    pub const DEFAULT_TENANT_ID: &'static str = "test-tenant-id";

    pub async fn start(port: u16) -> Result<Self> {
        let listener = std::net::TcpListener::bind(format!("127.0.0.1:{port}"))
            .context("Failed to bind MockTeams port")?;

        let server = MockServer::builder().listener(listener).start().await;
        let service_url = format!("http://127.0.0.1:{port}");

        let mock = Self {
            server,
            port,
            expectations: new_expectation_store(),
            sent_activities: Arc::new(Mutex::new(Vec::new())),
            conversations: Arc::new(Mutex::new(HashMap::new())),
            bot_id: Self::DEFAULT_BOT_ID.to_string(),
            bot_name: Self::DEFAULT_BOT_NAME.to_string(),
            tenant_id: Self::DEFAULT_TENANT_ID.to_string(),
            service_url,
        };

        mock.setup_default_routes().await;

        Ok(mock)
    }

    pub async fn start_with_config(
        port: u16,
        bot_id: &str,
        bot_name: &str,
        tenant_id: &str,
    ) -> Result<Self> {
        let listener = std::net::TcpListener::bind(format!("127.0.0.1:{port}"))
            .context("Failed to bind MockTeams port")?;

        let server = MockServer::builder().listener(listener).start().await;
        let service_url = format!("http://127.0.0.1:{port}");

        let mock = Self {
            server,
            port,
            expectations: new_expectation_store(),
            sent_activities: Arc::new(Mutex::new(Vec::new())),
            conversations: Arc::new(Mutex::new(HashMap::new())),
            bot_id: bot_id.to_string(),
            bot_name: bot_name.to_string(),
            tenant_id: tenant_id.to_string(),
            service_url,
        };

        mock.setup_default_routes().await;

        Ok(mock)
    }

    async fn setup_default_routes(&self) {
        let sent_activities = self.sent_activities.clone();

        Mock::given(method("POST"))
            .and(path_regex(r"/v3/conversations/.+/activities"))
            .respond_with(move |req: &wiremock::Request| {
                let body: serde_json::Value = req.body_json().unwrap_or_default();

                let activity = Activity {
                    kind: body
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("message")
                        .to_string(),
                    id: Uuid::new_v4().to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    local_timestamp: None,
                    service_url: String::new(),
                    channel_id: "msteams".to_string(),
                    from: ChannelAccount::default(),
                    conversation: ConversationAccount::default(),
                    recipient: ChannelAccount::default(),
                    text: body.get("text").and_then(|v| v.as_str()).map(String::from),
                    text_format: None,
                    locale: None,
                    attachments: None,
                    entities: None,
                    channel_data: None,
                    action: None,
                    reply_to_id: None,
                    value: None,
                    name: None,
                };

                sent_activities.lock().unwrap().push(activity.clone());

                let response = ResourceResponse { id: activity.id };

                ResponseTemplate::new(200).set_body_json(&response)
            })
            .mount(&self.server)
            .await;

        Mock::given(method("POST"))
            .and(path_regex(r"/v3/conversations/.+/activities/.+"))
            .respond_with(|_req: &wiremock::Request| {
                let response = ResourceResponse {
                    id: Uuid::new_v4().to_string(),
                };
                ResponseTemplate::new(200).set_body_json(&response)
            })
            .mount(&self.server)
            .await;

        Mock::given(method("PUT"))
            .and(path_regex(r"/v3/conversations/.+/activities/.+"))
            .respond_with(|_req: &wiremock::Request| {
                let response = ResourceResponse {
                    id: Uuid::new_v4().to_string(),
                };
                ResponseTemplate::new(200).set_body_json(&response)
            })
            .mount(&self.server)
            .await;

        Mock::given(method("DELETE"))
            .and(path_regex(r"/v3/conversations/.+/activities/.+"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&self.server)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/v3/conversations/.+/members"))
            .respond_with(|_req: &wiremock::Request| {
                let members = vec![TeamsChannelAccount {
                    id: "user-1".to_string(),
                    name: Some("Test User".to_string()),
                    aad_object_id: Some(Uuid::new_v4().to_string()),
                    email: Some("testuser@example.com".to_string()),
                    user_principal_name: Some("testuser@example.com".to_string()),
                    tenant_id: Some("test-tenant".to_string()),
                    given_name: Some("Test".to_string()),
                    surname: Some("User".to_string()),
                }];
                ResponseTemplate::new(200).set_body_json(&members)
            })
            .mount(&self.server)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/v3/conversations/.+/members/.+"))
            .respond_with(|_req: &wiremock::Request| {
                let member = TeamsChannelAccount {
                    id: "user-1".to_string(),
                    name: Some("Test User".to_string()),
                    aad_object_id: Some(Uuid::new_v4().to_string()),
                    email: Some("testuser@example.com".to_string()),
                    user_principal_name: Some("testuser@example.com".to_string()),
                    tenant_id: Some("test-tenant".to_string()),
                    given_name: Some("Test".to_string()),
                    surname: Some("User".to_string()),
                };
                ResponseTemplate::new(200).set_body_json(&member)
            })
            .mount(&self.server)
            .await;

        Mock::given(method("POST"))
            .and(path("/v3/conversations"))
            .respond_with(|_req: &wiremock::Request| {
                let conversation = ConversationAccount {
                    id: format!("conv-{}", Uuid::new_v4()),
                    name: None,
                    conversation_type: Some("personal".to_string()),
                    is_group: Some(false),
                    tenant_id: Some("test-tenant".to_string()),
                };
                ResponseTemplate::new(200).set_body_json(&conversation)
            })
            .mount(&self.server)
            .await;

        Mock::given(method("GET"))
            .and(path("/v3/conversations"))
            .respond_with(|_req: &wiremock::Request| {
                let result = ConversationsResult {
                    continuation_token: None,
                    conversations: vec![],
                };
                ResponseTemplate::new(200).set_body_json(&result)
            })
            .mount(&self.server)
            .await;

        Mock::given(method("POST"))
            .and(path("/botframework.com/oauth2/v2.0/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "token_type": "Bearer",
                "expires_in": 3600,
                "access_token": format!("test_token_{}", Uuid::new_v4())
            })))
            .mount(&self.server)
            .await;
    }

    #[must_use]
    pub fn simulate_message(&self, from_id: &str, from_name: &str, text: &str) -> Activity {
        let conversation_id = format!("conv-{}", Uuid::new_v4());

        Activity {
            kind: "message".to_string(),
            id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            local_timestamp: Some(chrono::Utc::now().to_rfc3339()),
            service_url: self.service_url.clone(),
            channel_id: "msteams".to_string(),
            from: ChannelAccount {
                id: from_id.to_string(),
                name: Some(from_name.to_string()),
                aad_object_id: Some(Uuid::new_v4().to_string()),
                role: Some("user".to_string()),
            },
            conversation: ConversationAccount {
                id: conversation_id,
                name: None,
                conversation_type: Some("personal".to_string()),
                is_group: Some(false),
                tenant_id: Some(self.tenant_id.clone()),
            },
            recipient: ChannelAccount {
                id: self.bot_id.clone(),
                name: Some(self.bot_name.clone()),
                aad_object_id: None,
                role: Some("bot".to_string()),
            },
            text: Some(text.to_string()),
            text_format: Some("plain".to_string()),
            locale: Some("en-US".to_string()),
            attachments: None,
            entities: None,
            channel_data: Some(serde_json::json!({
                "tenant": {
                    "id": self.tenant_id
                }
            })),
            action: None,
            reply_to_id: None,
            value: None,
            name: None,
        }
    }

    #[must_use]
    pub fn simulate_mention(&self, from_id: &str, from_name: &str, text: &str) -> Activity {
        let mut activity = self.simulate_message(from_id, from_name, text);

        let mention_text = format!("<at>{}</at>", self.bot_name);
        activity.text = Some(format!("{mention_text} {text}"));

        activity.entities = Some(vec![Entity {
            kind: "mention".to_string(),
            mentioned: Some(ChannelAccount {
                id: self.bot_id.clone(),
                name: Some(self.bot_name.clone()),
                aad_object_id: None,
                role: None,
            }),
            text: Some(mention_text),
            additional: HashMap::new(),
        }]);

        activity
    }

    #[must_use]
    pub fn simulate_member_added(&self, member_id: &str, member_name: &str) -> Activity {
        let conversation_id = format!("conv-{}", Uuid::new_v4());

        Activity {
            kind: "conversationUpdate".to_string(),
            id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            local_timestamp: None,
            service_url: self.service_url.clone(),
            channel_id: "msteams".to_string(),
            from: ChannelAccount {
                id: member_id.to_string(),
                name: Some(member_name.to_string()),
                aad_object_id: None,
                role: None,
            },
            conversation: ConversationAccount {
                id: conversation_id,
                name: None,
                conversation_type: Some("personal".to_string()),
                is_group: Some(false),
                tenant_id: Some(self.tenant_id.clone()),
            },
            recipient: ChannelAccount {
                id: self.bot_id.clone(),
                name: Some(self.bot_name.clone()),
                aad_object_id: None,
                role: Some("bot".to_string()),
            },
            text: None,
            text_format: None,
            locale: None,
            attachments: None,
            entities: None,
            channel_data: Some(serde_json::json!({
                "tenant": {
                    "id": self.tenant_id
                },
                "eventType": "teamMemberAdded"
            })),
            action: Some("add".to_string()),
            reply_to_id: None,
            value: None,
            name: None,
        }
    }

    #[must_use]
    pub fn simulate_invoke(
        &self,
        from_id: &str,
        from_name: &str,
        name: &str,
        value: serde_json::Value,
    ) -> Activity {
        let conversation_id = format!("conv-{}", Uuid::new_v4());

        Activity {
            kind: "invoke".to_string(),
            id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            local_timestamp: None,
            service_url: self.service_url.clone(),
            channel_id: "msteams".to_string(),
            from: ChannelAccount {
                id: from_id.to_string(),
                name: Some(from_name.to_string()),
                aad_object_id: Some(Uuid::new_v4().to_string()),
                role: Some("user".to_string()),
            },
            conversation: ConversationAccount {
                id: conversation_id,
                name: None,
                conversation_type: Some("personal".to_string()),
                is_group: Some(false),
                tenant_id: Some(self.tenant_id.clone()),
            },
            recipient: ChannelAccount {
                id: self.bot_id.clone(),
                name: Some(self.bot_name.clone()),
                aad_object_id: None,
                role: Some("bot".to_string()),
            },
            text: None,
            text_format: None,
            locale: Some("en-US".to_string()),
            attachments: None,
            entities: None,
            channel_data: Some(serde_json::json!({
                "tenant": {
                    "id": self.tenant_id
                }
            })),
            action: None,
            reply_to_id: None,
            value: Some(value),
            name: Some(name.to_string()),
        }
    }

    #[must_use]
    pub fn simulate_adaptive_card_action(
        &self,
        from_id: &str,
        from_name: &str,
        action_data: serde_json::Value,
    ) -> Activity {
        self.simulate_invoke(
            from_id,
            from_name,
            "adaptiveCard/action",
            serde_json::json!({
                "action": {
                    "type": "Action.Execute",
                    "verb": "submitAction",
                    "data": action_data
                }
            }),
        )
    }

    #[must_use]
    pub fn simulate_reaction(
        &self,
        from_id: &str,
        from_name: &str,
        message_id: &str,
        reaction: &str,
    ) -> Activity {
        let conversation_id = format!("conv-{}", Uuid::new_v4());

        Activity {
            kind: "messageReaction".to_string(),
            id: Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            local_timestamp: None,
            service_url: self.service_url.clone(),
            channel_id: "msteams".to_string(),
            from: ChannelAccount {
                id: from_id.to_string(),
                name: Some(from_name.to_string()),
                aad_object_id: None,
                role: None,
            },
            conversation: ConversationAccount {
                id: conversation_id,
                name: None,
                conversation_type: Some("personal".to_string()),
                is_group: Some(false),
                tenant_id: Some(self.tenant_id.clone()),
            },
            recipient: ChannelAccount {
                id: self.bot_id.clone(),
                name: Some(self.bot_name.clone()),
                aad_object_id: None,
                role: Some("bot".to_string()),
            },
            text: None,
            text_format: None,
            locale: None,
            attachments: None,
            entities: None,
            channel_data: None,
            action: None,
            reply_to_id: Some(message_id.to_string()),
            value: Some(serde_json::json!({
                "reactionsAdded": [{
                    "type": reaction
                }]
            })),
            name: None,
        }
    }

    pub async fn expect_error(&self, code: &str, message: &str) {
        let error_response = ErrorResponse {
            error: ErrorBody {
                code: code.to_string(),
                message: message.to_string(),
            },
        };

        Mock::given(method("POST"))
            .and(path_regex(r"/v3/conversations/.+/activities"))
            .respond_with(ResponseTemplate::new(400).set_body_json(&error_response))
            .mount(&self.server)
            .await;
    }

    pub async fn expect_unauthorized(&self) {
        self.expect_error("Unauthorized", "Token validation failed")
            .await;
    }

    pub async fn expect_not_found(&self) {
        self.expect_error("NotFound", "Conversation not found")
            .await;
    }

    #[must_use]
    pub fn sent_activities(&self) -> Vec<Activity> {
        self.sent_activities.lock().unwrap().clone()
    }

    #[must_use]
    pub fn sent_activities_containing(&self, text: &str) -> Vec<Activity> {
        self.sent_activities
            .lock()
            .unwrap()
            .iter()
            .filter(|a| a.text.as_ref().is_some_and(|t| t.contains(text)))
            .cloned()
            .collect()
    }

    #[must_use]
    pub fn last_sent_activity(&self) -> Option<Activity> {
        self.sent_activities.lock().unwrap().last().cloned()
    }

    pub fn clear_sent_activities(&self) {
        self.sent_activities.lock().unwrap().clear();
    }

    pub fn register_conversation(&self, info: ConversationInfo) {
        self.conversations
            .lock()
            .unwrap()
            .insert(info.id.clone(), info);
    }

    #[must_use]
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    #[must_use]
    pub fn service_url(&self) -> String {
        self.service_url.clone()
    }

    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    #[must_use]
    pub fn bot_id(&self) -> &str {
        &self.bot_id
    }

    #[must_use]
    pub fn bot_name(&self) -> &str {
        &self.bot_name
    }

    #[must_use]
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    pub fn verify(&self) -> Result<()> {
        let store = self.expectations.lock().unwrap();
        for (_, exp) in store.iter() {
            exp.verify()?;
        }
        Ok(())
    }

    pub async fn reset(&self) {
        self.server.reset().await;
        self.sent_activities.lock().unwrap().clear();
        self.conversations.lock().unwrap().clear();
        self.expectations.lock().unwrap().clear();
        self.setup_default_routes().await;
    }

    pub async fn received_requests(&self) -> Vec<wiremock::Request> {
        self.server.received_requests().await.unwrap_or_default()
    }
}

/// Test-only helper: build an Adaptive Card attachment.
#[cfg(test)]
pub fn adaptive_card(content: serde_json::Value) -> Attachment {
    Attachment {
        content_type: "application/vnd.microsoft.card.adaptive".to_string(),
        content_url: None,
        content: Some(content),
        name: None,
        thumbnail_url: None,
    }
}

/// Test-only helper: build a Hero Card attachment.
#[cfg(test)]
pub fn hero_card(title: &str, subtitle: Option<&str>, text: Option<&str>) -> Attachment {
    Attachment {
        content_type: "application/vnd.microsoft.card.hero".to_string(),
        content_url: None,
        content: Some(serde_json::json!({
            "title": title,
            "subtitle": subtitle,
            "text": text
        })),
        name: None,
        thumbnail_url: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_default() {
        let activity = Activity::default();
        assert_eq!(activity.kind, "message");
        assert_eq!(activity.channel_id, "msteams");
        assert!(!activity.id.is_empty());
    }

    #[test]
    fn test_activity_serialization() {
        let activity = Activity {
            kind: "message".to_string(),
            id: "test-id".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            local_timestamp: None,
            service_url: "http://localhost".to_string(),
            channel_id: "msteams".to_string(),
            from: ChannelAccount {
                id: "user-1".to_string(),
                name: Some("Test User".to_string()),
                aad_object_id: None,
                role: None,
            },
            conversation: ConversationAccount {
                id: "conv-1".to_string(),
                name: None,
                conversation_type: Some("personal".to_string()),
                is_group: Some(false),
                tenant_id: Some("tenant-1".to_string()),
            },
            recipient: ChannelAccount::default(),
            text: Some("Hello!".to_string()),
            text_format: None,
            locale: None,
            attachments: None,
            entities: None,
            channel_data: None,
            action: None,
            reply_to_id: None,
            value: None,
            name: None,
        };

        let json = serde_json::to_string(&activity).unwrap();
        assert!(json.contains("Hello!"));
        assert!(json.contains("msteams"));
        assert!(json.contains("Test User"));
    }

    #[test]
    fn test_resource_response() {
        let response = ResourceResponse {
            id: "msg-123".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("msg-123"));
    }

    #[test]
    fn test_adaptive_card_helper() {
        let card = adaptive_card(serde_json::json!({
            "type": "AdaptiveCard",
            "body": [{"type": "TextBlock", "text": "Hello"}]
        }));

        assert_eq!(card.content_type, "application/vnd.microsoft.card.adaptive");
        assert!(card.content.is_some());
    }

    #[test]
    fn test_hero_card_helper() {
        let card = hero_card("Title", Some("Subtitle"), Some("Text"));

        assert_eq!(card.content_type, "application/vnd.microsoft.card.hero");
        let content = card.content.unwrap();
        assert_eq!(content["title"], "Title");
    }

    #[test]
    fn test_entity_mention() {
        let entity = Entity {
            kind: "mention".to_string(),
            mentioned: Some(ChannelAccount {
                id: "bot-id".to_string(),
                name: Some("Bot".to_string()),
                aad_object_id: None,
                role: None,
            }),
            text: Some("<at>Bot</at>".to_string()),
            additional: HashMap::new(),
        };

        let json = serde_json::to_string(&entity).unwrap();
        assert!(json.contains("mention"));
        assert!(json.contains("<at>Bot</at>"));
    }

    #[test]
    fn test_error_response() {
        let error = ErrorResponse {
            error: ErrorBody {
                code: "BadRequest".to_string(),
                message: "Invalid activity".to_string(),
            },
        };

        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("BadRequest"));
        assert!(json.contains("Invalid activity"));
    }
}
