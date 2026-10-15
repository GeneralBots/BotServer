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

mod part2;
mod part3;

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
