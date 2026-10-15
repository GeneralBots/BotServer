//! Split from `teams.rs` (#1504).

use super::*;

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

    pub(crate) async fn setup_default_routes(&self) {
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
}
