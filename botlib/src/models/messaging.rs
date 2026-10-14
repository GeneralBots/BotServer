use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::message_types::MessageType;

/// A lightweight @-mention reference carried on a chat message (#939).
///
/// `kind` selects the entity plane - `"integration"` resolves against the
/// connection control plane; other kinds are reserved for future surfaces.
/// Both fields default through serde so payloads written before mentions
/// existed deserialize unchanged, and unknown extra members are ignored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MentionRef {
    pub kind: String,
    #[serde(default)]
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub bot_id: String,
    pub user_id: String,
    pub session_id: String,
    pub channel: String,
    pub content: String,
    pub message_type: MessageType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_url: Option<String>,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_switchers: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mentions: Vec<MentionRef>,
}

impl UserMessage {
    #[must_use]
    pub fn text(
        bot_id: impl Into<String>,
        user_id: impl Into<String>,
        session_id: impl Into<String>,
        channel: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            bot_id: bot_id.into(),
            user_id: user_id.into(),
            session_id: session_id.into(),
            channel: channel.into(),
            content: content.into(),
            message_type: MessageType::USER,
            media_url: None,
            timestamp: Utc::now(),
            context_name: None,
            active_switchers: Vec::new(),
            mentions: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_media(mut self, url: impl Into<String>) -> Self {
        self.media_url = Some(url.into());
        self
    }

    #[must_use]
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context_name = Some(context.into());
        self
    }

    #[must_use]
    pub const fn has_media(&self) -> bool {
        self.media_url.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

impl Suggestion {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            context: None,
            action: None,
            icon: None,
        }
    }

    #[must_use]
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    #[must_use]
    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }

    #[must_use]
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

impl<S: Into<String>> From<S> for Suggestion {
    fn from(text: S) -> Self {
        Self::new(text)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Switcher {
    pub id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

impl Switcher {
    #[must_use]
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            prompt: None,
            color: None,
            icon: None,
        }
    }

    #[must_use]
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    #[must_use]
    pub fn with_color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    #[must_use]
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotResponse {
    pub bot_id: String,
    pub user_id: String,
    pub session_id: String,
    pub channel: String,
    pub content: String,
    pub message_type: MessageType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_token: Option<String>,
    pub is_complete: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<Suggestion>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub switchers: Vec<Switcher>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_name: Option<String>,
    #[serde(default)]
    pub context_length: usize,
    #[serde(default)]
    pub context_max_length: usize,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reasoning: String,
}

impl BotResponse {
    #[must_use]
    pub fn new(
        bot_id: impl Into<String>,
        session_id: impl Into<String>,
        user_id: impl Into<String>,
        content: impl Into<String>,
        channel: impl Into<String>,
    ) -> Self {
        Self {
            bot_id: bot_id.into(),
            user_id: user_id.into(),
            session_id: session_id.into(),
            channel: channel.into(),
            content: content.into(),
            message_type: MessageType::BOT_RESPONSE,
            stream_token: None,
            is_complete: true,
            suggestions: Vec::new(),
            switchers: Vec::new(),
            context_name: None,
            context_length: 0,
            context_max_length: 0,
            reasoning: String::new(),
        }
    }

    #[must_use]
    pub fn streaming(
        bot_id: impl Into<String>,
        session_id: impl Into<String>,
        user_id: impl Into<String>,
        channel: impl Into<String>,
        stream_token: impl Into<String>,
    ) -> Self {
        Self {
            bot_id: bot_id.into(),
            user_id: user_id.into(),
            session_id: session_id.into(),
            channel: channel.into(),
            content: String::new(),
            message_type: MessageType::BOT_RESPONSE,
            stream_token: Some(stream_token.into()),
            is_complete: false,
            suggestions: Vec::new(),
            switchers: Vec::new(),
            context_name: None,
            context_length: 0,
            context_max_length: 0,
            reasoning: String::new(),
        }
    }

    #[must_use]
    pub fn with_suggestions<I, S>(mut self, suggestions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Suggestion>,
    {
        self.suggestions = suggestions.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub fn add_suggestion(mut self, suggestion: impl Into<Suggestion>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }

    #[must_use]
    pub fn with_context(
        mut self,
        name: impl Into<String>,
        length: usize,
        max_length: usize,
    ) -> Self {
        self.context_name = Some(name.into());
        self.context_length = length;
        self.context_max_length = max_length;
        self
    }

    pub fn append_content(&mut self, chunk: &str) {
        self.content.push_str(chunk);
    }

    #[must_use]
    pub const fn complete(mut self) -> Self {
        self.is_complete = true;
        self
    }

    #[must_use]
    pub const fn is_streaming(&self) -> bool {
        self.stream_token.is_some() && !self.is_complete
    }

    #[must_use]
    pub const fn has_suggestions(&self) -> bool {
        !self.suggestions.is_empty()
    }
}

impl Default for BotResponse {
    fn default() -> Self {
        Self {
            bot_id: String::new(),
            user_id: String::new(),
            session_id: String::new(),
            channel: String::new(),
            content: String::new(),
            message_type: MessageType::BOT_RESPONSE,
            stream_token: None,
            is_complete: true,
            suggestions: Vec::new(),
            switchers: Vec::new(),
            context_name: None,
            context_length: 0,
            context_max_length: 0,
            reasoning: String::new(),
        }
    }
}

#[cfg(test)]
mod mention_tests {
    use super::*;

    #[test]
    fn payload_without_mentions_deserializes_with_empty_vec() {
        let legacy = serde_json::json!({
            "bot_id": "bot",
            "user_id": "user",
            "session_id": "session",
            "channel": "web",
            "content": "hello",
            "message_type": 1,
            "timestamp": "2026-01-01T00:00:00Z"
        });
        let parsed: UserMessage = serde_json::from_value(legacy).expect("legacy payload in tests");
        assert!(parsed.mentions.is_empty());
    }

    #[test]
    fn mentions_round_trip_and_skip_serialization_when_empty() {
        let message = UserMessage::text("bot", "user", "sess", "web", "use @integration:aws");
        assert_eq!(message.mentions.len(), 0);
        let without = serde_json::to_string(&message).expect("serialize in tests");
        assert!(!without.contains("\"mentions\""));

        let referenced = UserMessage {
            mentions: vec![MentionRef {
                kind: "integration".to_string(),
                id: "0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0".to_string(),
                label: Some("AWS".to_string()),
            }],
            ..UserMessage::text("bot", "user", "sess", "web", "use @integration:aws")
        };
        let serialized = serde_json::to_string(&referenced).expect("serialize in tests");
        let parsed: UserMessage = serde_json::from_str(&serialized).expect("round trip in tests");
        assert_eq!(parsed.mentions.len(), 1);
        assert_eq!(parsed.mentions[0].kind, "integration");
        assert_eq!(
            parsed.mentions[0].id,
            "0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0"
        );
        assert_eq!(parsed.mentions[0].label.as_deref(), Some("AWS"));
    }

    #[test]
    fn partial_mention_entries_fill_defaults() {
        let lenient = serde_json::json!({
            "bot_id": "bot",
            "user_id": "user",
            "session_id": "session",
            "channel": "web",
            "content": "hi",
            "message_type": 1,
            "timestamp": "2026-01-01T00:00:00Z",
            "mentions": [{ "kind": "integration" }]
        });
        let parsed: UserMessage = serde_json::from_value(lenient).expect("lenient payload");
        assert_eq!(parsed.mentions.len(), 1);
        assert_eq!(parsed.mentions[0].kind, "integration");
        assert_eq!(parsed.mentions[0].id, "");
        assert!(parsed.mentions[0].label.is_none());
    }
}
