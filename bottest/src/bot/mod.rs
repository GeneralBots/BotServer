
pub mod conversation;
pub mod runner;

use crate::fixtures::MessageDirection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotResponse {
    pub id: Uuid,
    pub content: String,
    pub content_type: ResponseContentType,
    pub metadata: HashMap<String, serde_json::Value>,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ResponseContentType {
    #[default]
    Text,
    Image,
    Audio,
    Video,
    Document,
    Interactive,
    Template,
    Location,
    Contact,
}


#[derive(Debug, Clone)]
pub struct AssertionResult {
    pub passed: bool,
    pub message: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

impl AssertionResult {
    #[must_use]
    pub fn pass(message: &str) -> Self {
        Self {
            passed: true,
            message: message.to_string(),
            expected: None,
            actual: None,
        }
    }

    #[must_use]
    pub fn fail(message: &str, expected: &str, actual: &str) -> Self {
        Self {
            passed: false,
            message: message.to_string(),
            expected: Some(expected.to_string()),
            actual: Some(actual.to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConversationConfig {
    pub response_timeout: Duration,
    pub record: bool,
    pub use_mock_llm: bool,
    pub variables: HashMap<String, String>,
}

impl Default for ConversationConfig {
    fn default() -> Self {
        Self {
            response_timeout: Duration::from_secs(30),
            record: true,
            use_mock_llm: true,
            variables: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationRecord {
    pub id: Uuid,
    pub bot_name: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub ended_at: Option<chrono::DateTime<chrono::Utc>>,
    pub messages: Vec<RecordedMessage>,
    pub assertions: Vec<AssertionRecord>,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedMessage {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub direction: MessageDirection,
    pub content: String,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionRecord {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub assertion_type: String,
    pub passed: bool,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum ConversationState {
    #[default]
    Initial,
    WaitingForUser,
    WaitingForBot,
    Transferred,
    Ended,
    Error,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assertion_result_pass() {
        let result = AssertionResult::pass("Test passed");
        assert!(result.passed);
        assert_eq!(result.message, "Test passed");
    }

    #[test]
    fn test_assertion_result_fail() {
        let result = AssertionResult::fail("Test failed", "expected", "actual");
        assert!(!result.passed);
        assert_eq!(result.expected, Some("expected".to_string()));
        assert_eq!(result.actual, Some("actual".to_string()));
    }

    #[test]
    fn test_conversation_config_default() {
        let config = ConversationConfig::default();
        assert_eq!(config.response_timeout, Duration::from_secs(30));
        assert!(config.record);
        assert!(config.use_mock_llm);
    }

    #[test]
    fn test_conversation_state_default() {
        let state = ConversationState::default();
        assert_eq!(state, ConversationState::Initial);
    }

    #[test]
    fn test_bot_response_serialization() {
        let response = BotResponse {
            id: Uuid::new_v4(),
            content: "Hello!".to_string(),
            content_type: ResponseContentType::Text,
            metadata: HashMap::new(),
            latency_ms: 150,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Hello!"));
        assert!(json.contains("text"));
    }
}
