//! Split from `whatsapp.rs` (#1504).

use super::*;

impl MockWhatsApp {
    pub const DEFAULT_PHONE_NUMBER_ID: &'static str = "123456789012345";

    pub const DEFAULT_BUSINESS_ACCOUNT_ID: &'static str = "987654321098765";

    pub const DEFAULT_ACCESS_TOKEN: &'static str = "test_access_token_12345";

    pub async fn start(port: u16) -> Result<Self> {
        let listener = std::net::TcpListener::bind(format!("127.0.0.1:{port}"))
            .context("Failed to bind MockWhatsApp port")?;

        let server = MockServer::builder().listener(listener).start().await;

        let mock = Self {
            server,
            port,
            expectations: new_expectation_store(),
            sent_messages: Arc::new(Mutex::new(Vec::new())),
            received_webhooks: Arc::new(Mutex::new(Vec::new())),
            phone_number_id: Self::DEFAULT_PHONE_NUMBER_ID.to_string(),
            business_account_id: Self::DEFAULT_BUSINESS_ACCOUNT_ID.to_string(),
            access_token: Self::DEFAULT_ACCESS_TOKEN.to_string(),
        };

        mock.setup_default_routes().await;

        Ok(mock)
    }

    pub async fn start_with_config(
        port: u16,
        phone_number_id: &str,
        business_account_id: &str,
        access_token: &str,
    ) -> Result<Self> {
        let listener = std::net::TcpListener::bind(format!("127.0.0.1:{port}"))
            .context("Failed to bind MockWhatsApp port")?;

        let server = MockServer::builder().listener(listener).start().await;

        let mock = Self {
            server,
            port,
            expectations: new_expectation_store(),
            sent_messages: Arc::new(Mutex::new(Vec::new())),
            received_webhooks: Arc::new(Mutex::new(Vec::new())),
            phone_number_id: phone_number_id.to_string(),
            business_account_id: business_account_id.to_string(),
            access_token: access_token.to_string(),
        };

        mock.setup_default_routes().await;

        Ok(mock)
    }

    pub(crate) async fn setup_default_routes(&self) {
        let sent_messages = self.sent_messages.clone();

        Mock::given(method("POST"))
            .and(path_regex(r"/v\d+\.\d+/\d+/messages"))
            .respond_with(move |req: &wiremock::Request| {
                let body: serde_json::Value = req.body_json().unwrap_or_default();
                let to = body.get("to").and_then(|v| v.as_str()).unwrap_or("unknown");
                let msg_type = body.get("type").and_then(|v| v.as_str()).unwrap_or("text");

                let message_id = format!("wamid.{}", Uuid::new_v4().to_string().replace('-', ""));

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                let content = match msg_type {
                    "text" => {
                        let text_body = body
                            .get("text")
                            .and_then(|t| t.get("body"))
                            .and_then(|b| b.as_str())
                            .unwrap_or("")
                            .to_string();
                        MessageContent::Text { body: text_body }
                    }
                    "template" => {
                        let template = body.get("template").unwrap_or(&serde_json::Value::Null);
                        let name = template
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("")
                            .to_string();
                        let lang = template
                            .get("language")
                            .and_then(|l| l.get("code"))
                            .and_then(|c| c.as_str())
                            .unwrap_or("en")
                            .to_string();
                        let components = template
                            .get("components")
                            .and_then(|c| c.as_array())
                            .cloned()
                            .unwrap_or_default();
                        MessageContent::Template {
                            name,
                            language: lang,
                            components,
                        }
                    }
                    _ => MessageContent::Text {
                        body: "unknown".to_string(),
                    },
                };

                let sent = SentMessage {
                    id: message_id.clone(),
                    to: to.to_string(),
                    message_type: match msg_type {
                        "template" => MessageType::Template,
                        "image" => MessageType::Image,
                        "document" => MessageType::Document,
                        "audio" => MessageType::Audio,
                        "video" => MessageType::Video,
                        "location" => MessageType::Location,
                        "interactive" => MessageType::Interactive,
                        "reaction" => MessageType::Reaction,
                        _ => MessageType::Text,
                    },
                    content,
                    timestamp: now,
                };

                sent_messages.lock().unwrap().push(sent);

                let response = SendMessageResponse {
                    messaging_product: "whatsapp".to_string(),
                    contacts: vec![ContactResponse {
                        input: to.to_string(),
                        wa_id: to.to_string(),
                    }],
                    messages: vec![MessageResponse { id: message_id }],
                };

                ResponseTemplate::new(200).set_body_json(&response)
            })
            .mount(&self.server)
            .await;

        Mock::given(method("POST"))
            .and(path_regex(r"/v\d+\.\d+/\d+/media"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": format!("media_{}", Uuid::new_v4())
            })))
            .mount(&self.server)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/v\d+\.\d+/\d+"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "url": "https://example.com/media/file.jpg",
                "mime_type": "image/jpeg",
                "sha256": "abc123",
                "file_size": 12345,
                "id": "media_123"
            })))
            .mount(&self.server)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/v\d+\.\d+/\d+/whatsapp_business_profile"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{
                    "messaging_product": "whatsapp",
                    "address": "123 Test St",
                    "description": "Test Business",
                    "vertical": "OTHER",
                    "email": "test@example.com",
                    "websites": ["https://example.com"],
                    "profile_picture_url": "https://example.com/pic.jpg"
                }]
            })))
            .mount(&self.server)
            .await;
    }

    #[must_use]
    pub fn expect_send_message(&self) -> MessageExpectation {
        let _ = self;
        MessageExpectation {
            message_type: None,
            contains: None,
        }
    }

    #[must_use]
    pub fn expect_send_template(&self) -> TemplateExpectation {
        let _ = self;
        TemplateExpectation {
            to: None,
            language: None,
        }
    }

    pub fn simulate_incoming(&self, from: &str, text: &str) -> Result<WebhookEvent> {
        let message_id = format!("wamid.{}", Uuid::new_v4().to_string().replace('-', ""));
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();

        let event = WebhookEvent {
            object: "whatsapp_business_account".to_string(),
            entry: vec![WebhookEntry {
                id: self.business_account_id.clone(),
                changes: vec![WebhookChange {
                    value: WebhookValue {
                        messaging_product: "whatsapp".to_string(),
                        metadata: WebhookMetadata {
                            display_phone_number: "15551234567".to_string(),
                            phone_number_id: self.phone_number_id.clone(),
                        },
                        contacts: Some(vec![WebhookContact {
                            profile: ContactProfile {
                                name: "Test User".to_string(),
                            },
                            wa_id: from.to_string(),
                        }]),
                        messages: Some(vec![IncomingMessage {
                            from: from.to_string(),
                            id: message_id,
                            timestamp,
                            message_type: "text".to_string(),
                            text: Some(TextMessage {
                                body: text.to_string(),
                            }),
                            image: None,
                            document: None,
                            button: None,
                            interactive: None,
                        }]),
                        statuses: None,
                    },
                    field: "messages".to_string(),
                }],
            }],
        };

        self.received_webhooks.lock().unwrap().push(event.clone());
        Ok(event)
    }

    pub fn simulate_incoming_image(
        &self,
        from: &str,
        media_id: &str,
        caption: Option<&str>,
    ) -> Result<WebhookEvent> {
        let message_id = format!("wamid.{}", Uuid::new_v4().to_string().replace('-', ""));
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();

        let event = WebhookEvent {
            object: "whatsapp_business_account".to_string(),
            entry: vec![WebhookEntry {
                id: self.business_account_id.clone(),
                changes: vec![WebhookChange {
                    value: WebhookValue {
                        messaging_product: "whatsapp".to_string(),
                        metadata: WebhookMetadata {
                            display_phone_number: "15551234567".to_string(),
                            phone_number_id: self.phone_number_id.clone(),
                        },
                        contacts: Some(vec![WebhookContact {
                            profile: ContactProfile {
                                name: "Test User".to_string(),
                            },
                            wa_id: from.to_string(),
                        }]),
                        messages: Some(vec![IncomingMessage {
                            from: from.to_string(),
                            id: message_id,
                            timestamp,
                            message_type: "image".to_string(),
                            text: None,
                            image: Some(MediaMessage {
                                id: Some(media_id.to_string()),
                                mime_type: Some("image/jpeg".to_string()),
                                sha256: Some("abc123".to_string()),
                                caption: caption.map(std::string::ToString::to_string),
                            }),
                            document: None,
                            button: None,
                            interactive: None,
                        }]),
                        statuses: None,
                    },
                    field: "messages".to_string(),
                }],
            }],
        };

        self.received_webhooks.lock().unwrap().push(event.clone());
        Ok(event)
    }

    pub fn simulate_button_reply(
        &self,
        from: &str,
        button_id: &str,
        button_text: &str,
    ) -> Result<WebhookEvent> {
        let message_id = format!("wamid.{}", Uuid::new_v4().to_string().replace('-', ""));
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();

        let event = WebhookEvent {
            object: "whatsapp_business_account".to_string(),
            entry: vec![WebhookEntry {
                id: self.business_account_id.clone(),
                changes: vec![WebhookChange {
                    value: WebhookValue {
                        messaging_product: "whatsapp".to_string(),
                        metadata: WebhookMetadata {
                            display_phone_number: "15551234567".to_string(),
                            phone_number_id: self.phone_number_id.clone(),
                        },
                        contacts: Some(vec![WebhookContact {
                            profile: ContactProfile {
                                name: "Test User".to_string(),
                            },
                            wa_id: from.to_string(),
                        }]),
                        messages: Some(vec![IncomingMessage {
                            from: from.to_string(),
                            id: message_id,
                            timestamp,
                            message_type: "interactive".to_string(),
                            text: None,
                            image: None,
                            document: None,
                            button: None,
                            interactive: Some(InteractiveReply {
                                reply_type: "button_reply".to_string(),
                                button_reply: Some(ButtonReplyContent {
                                    id: button_id.to_string(),
                                    title: button_text.to_string(),
                                }),
                                list_reply: None,
                            }),
                        }]),
                        statuses: None,
                    },
                    field: "messages".to_string(),
                }],
            }],
        };

        self.received_webhooks.lock().unwrap().push(event.clone());
        Ok(event)
    }

    pub fn simulate_webhook(&self, event: WebhookEvent) -> Result<()> {
        self.received_webhooks.lock().unwrap().push(event);
        Ok(())
    }

    pub fn simulate_status(
        &self,
        message_id: &str,
        status: &str,
        recipient: &str,
    ) -> Result<WebhookEvent> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();

        let event = WebhookEvent {
            object: "whatsapp_business_account".to_string(),
            entry: vec![WebhookEntry {
                id: self.business_account_id.clone(),
                changes: vec![WebhookChange {
                    value: WebhookValue {
                        messaging_product: "whatsapp".to_string(),
                        metadata: WebhookMetadata {
                            display_phone_number: "15551234567".to_string(),
                            phone_number_id: self.phone_number_id.clone(),
                        },
                        contacts: None,
                        messages: None,
                        statuses: Some(vec![MessageStatus {
                            id: message_id.to_string(),
                            status: status.to_string(),
                            timestamp,
                            recipient_id: recipient.to_string(),
                            conversation: Some(Conversation {
                                id: format!("conv_{}", Uuid::new_v4()),
                                origin: Some(ConversationOrigin {
                                    origin_type: "business_initiated".to_string(),
                                }),
                            }),
                            pricing: Some(Pricing {
                                billable: true,
                                model: "CBP".to_string(),
                                category: "business_initiated".to_string(),
                            }),
                        }]),
                    },
                    field: "messages".to_string(),
                }],
            }],
        };

        self.received_webhooks.lock().unwrap().push(event.clone());
        Ok(event)
    }
}
