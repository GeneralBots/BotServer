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

mod part2;
mod part3;

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
