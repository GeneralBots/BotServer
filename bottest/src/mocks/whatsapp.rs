use super::{new_expectation_store, ExpectationStore};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub struct MockWhatsApp {
    server: MockServer,
    port: u16,
    expectations: ExpectationStore,
    sent_messages: Arc<Mutex<Vec<SentMessage>>>,
    received_webhooks: Arc<Mutex<Vec<WebhookEvent>>>,
    phone_number_id: String,
    business_account_id: String,
    access_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentMessage {
    pub id: String,
    pub to: String,
    pub message_type: MessageType,
    pub content: MessageContent,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    Text,
    Template,
    Image,
    Document,
    Audio,
    Video,
    Location,
    Contacts,
    Interactive,
    Reaction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text {
        body: String,
    },
    Template {
        name: String,
        language: String,
        components: Vec<serde_json::Value>,
    },
    Media {
        url: String,
        caption: Option<String>,
    },
    Location {
        latitude: f64,
        longitude: f64,
        name: Option<String>,
    },
    Interactive {
        r#type: String,
        body: serde_json::Value,
    },
    Reaction {
        message_id: String,
        emoji: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub object: String,
    pub entry: Vec<WebhookEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEntry {
    pub id: String,
    pub changes: Vec<WebhookChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookChange {
    pub value: WebhookValue,
    pub field: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookValue {
    pub messaging_product: String,
    pub metadata: WebhookMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contacts: Option<Vec<WebhookContact>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<IncomingMessage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statuses: Option<Vec<MessageStatus>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookMetadata {
    pub display_phone_number: String,
    pub phone_number_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookContact {
    pub profile: ContactProfile,
    pub wa_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactProfile {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingMessage {
    pub from: String,
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "type")]
    pub message_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<TextMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<MediaMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<MediaMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub button: Option<ButtonReply>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interactive: Option<InteractiveReply>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextMessage {
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonReply {
    pub payload: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveReply {
    #[serde(rename = "type")]
    pub reply_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub button_reply: Option<ButtonReplyContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_reply: Option<ListReplyContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonReplyContent {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListReplyContent {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageStatus {
    pub id: String,
    pub status: String,
    pub timestamp: String,
    pub recipient_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation: Option<Conversation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing: Option<Pricing>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<ConversationOrigin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationOrigin {
    #[serde(rename = "type")]
    pub origin_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pricing {
    pub billable: bool,
    #[serde(alias = "pricing_model")]
    pub model: String,
    pub category: String,
}

#[derive(Serialize)]
struct SendMessageResponse {
    messaging_product: String,
    contacts: Vec<ContactResponse>,
    messages: Vec<MessageResponse>,
}

#[derive(Serialize)]
struct ContactResponse {
    input: String,
    wa_id: String,
}

#[derive(Serialize)]
struct MessageResponse {
    id: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
    code: u32,
    fbtrace_id: String,
}

pub struct MessageExpectation {
    message_type: Option<MessageType>,
    contains: Option<String>,
}

impl MessageExpectation {
    pub const fn of_type(mut self, t: MessageType) -> Self {
        self.message_type = Some(t);
        self
    }

    pub fn containing(mut self, text: &str) -> Self {
        self.contains = Some(text.to_string());
        self
    }
}

pub struct TemplateExpectation {
    to: Option<String>,
    language: Option<String>,
}

impl TemplateExpectation {
    pub fn to(mut self, phone: &str) -> Self {
        self.to = Some(phone.to_string());
        self
    }

    pub fn with_language(mut self, lang: &str) -> Self {
        self.language = Some(lang.to_string());
        self
    }
}

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

    async fn setup_default_routes(&self) {
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

    pub async fn expect_error(&self, code: u32, message: &str) {
        let error_response = ErrorResponse {
            error: ErrorDetail {
                message: message.to_string(),
                error_type: "OAuthException".to_string(),
                code,
                fbtrace_id: format!("trace_{}", Uuid::new_v4()),
            },
        };

        Mock::given(method("POST"))
            .and(path_regex(r"/v\d+\.\d+/\d+/messages"))
            .respond_with(ResponseTemplate::new(400).set_body_json(&error_response))
            .mount(&self.server)
            .await;
    }

    pub async fn expect_rate_limit(&self) {
        self.expect_error(80007, "Rate limit hit").await;
    }

    pub async fn expect_invalid_token(&self) {
        let error_response = ErrorResponse {
            error: ErrorDetail {
                message: "Invalid OAuth access token".to_string(),
                error_type: "OAuthException".to_string(),
                code: 190,
                fbtrace_id: format!("trace_{}", Uuid::new_v4()),
            },
        };

        Mock::given(method("POST"))
            .and(path_regex(r"/v\d+\.\d+/\d+/messages"))
            .respond_with(ResponseTemplate::new(401).set_body_json(&error_response))
            .mount(&self.server)
            .await;
    }

    #[must_use]
    pub fn sent_messages(&self) -> Vec<SentMessage> {
        self.sent_messages.lock().unwrap().clone()
    }

    #[must_use]
    pub fn sent_messages_to(&self, phone: &str) -> Vec<SentMessage> {
        self.sent_messages
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.to == phone)
            .cloned()
            .collect()
    }

    #[must_use]
    pub fn last_sent_message(&self) -> Option<SentMessage> {
        self.sent_messages.lock().unwrap().last().cloned()
    }

    pub fn clear_sent_messages(&self) {
        self.sent_messages.lock().unwrap().clear();
    }

    #[must_use]
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    #[must_use]
    pub fn graph_api_url(&self) -> String {
        format!("http://127.0.0.1:{}/v17.0", self.port)
    }

    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    #[must_use]
    pub fn phone_number_id(&self) -> &str {
        &self.phone_number_id
    }

    #[must_use]
    pub fn business_account_id(&self) -> &str {
        &self.business_account_id
    }

    #[must_use]
    pub fn access_token(&self) -> &str {
        &self.access_token
    }

    pub fn verify(&self) -> Result<()> {
        let store = self.expectations.lock().unwrap();
        for exp in store.values() {
            exp.verify()?;
        }
        Ok(())
    }

    pub async fn reset(&self) {
        self.server.reset().await;
        self.sent_messages.lock().unwrap().clear();
        self.received_webhooks.lock().unwrap().clear();
        self.expectations.lock().unwrap().clear();
        self.setup_default_routes().await;
    }

    pub async fn received_requests(&self) -> Vec<wiremock::Request> {
        self.server.received_requests().await.unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type_serialization() {
        let msg_type = MessageType::Template;
        let json = serde_json::to_string(&msg_type).unwrap();
        assert_eq!(json, "\"template\"");
    }

    #[test]
    fn test_webhook_event_serialization() {
        let event = WebhookEvent {
            object: "whatsapp_business_account".to_string(),
            entry: vec![],
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("whatsapp_business_account"));
    }

    #[test]
    fn test_incoming_message_text() {
        let msg = IncomingMessage {
            from: "15551234567".to_string(),
            id: "wamid.123".to_string(),
            timestamp: "1234567890".to_string(),
            message_type: "text".to_string(),
            text: Some(TextMessage {
                body: "Hello!".to_string(),
            }),
            image: None,
            document: None,
            button: None,
            interactive: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("Hello!"));
        assert!(json.contains("15551234567"));
    }

    #[test]
    fn test_message_status() {
        let status = MessageStatus {
            id: "wamid.123".to_string(),
            status: "delivered".to_string(),
            timestamp: "1234567890".to_string(),
            recipient_id: "15551234567".to_string(),
            conversation: None,
            pricing: None,
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("delivered"));
    }

    #[test]
    fn test_error_response() {
        let error = ErrorResponse {
            error: ErrorDetail {
                message: "Test error".to_string(),
                error_type: "OAuthException".to_string(),
                code: 100,
                fbtrace_id: "trace123".to_string(),
            },
        };

        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("Test error"));
        assert!(json.contains("100"));
    }
}
