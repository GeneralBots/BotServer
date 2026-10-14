

// Configuration matching Node.js proxy exactly

pub(crate) use async_trait::async_trait;
pub(crate) use futures_util::StreamExt;
pub(crate) use log::{error, trace, warn};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use serde_json::Value;
pub(crate) use std::time::Duration;
pub(crate) use tokio::sync::mpsc;
pub(crate) use tokio::time::Instant;
pub(crate) use super::{llm_models::get_handler, LLMProvider};

const MAX_RETRIES: u32 = 5;
const INITIAL_DELAY_MS: u64 = 1000;
const MAX_DELAY_MS: u64 = 30000;
const BACKOFF_MULTIPLIER: f64 = 2.0;
const TIMEOUT_MS: u64 = 60000;
const STREAMING_TIMEOUT_MS: u64 = 180000; // 3 minutes for streaming
const ACTIVITY_TIMEOUT_MS: u64 = 30000; // 30 seconds no data = timeout

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeThinking {
    #[serde(rename = "type")]
    pub thinking_type: String, // "enabled"
    pub budget_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<ClaudeMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ClaudeThinking>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeContentBlock {
    #[serde(rename = "type")]
    pub content_type: String,
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub role: String,
    pub content: Vec<ClaudeContentBlock>,
    pub model: String,
    #[serde(default)]
    pub stop_reason: Option<String>,
}

// SSE event structures - Anthropic format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeStreamDelta {
    #[serde(rename = "type", default)]
    pub delta_type: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub thinking: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeStreamEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub delta: Option<ClaudeStreamDelta>,
    #[serde(default)]
    pub index: Option<u32>,
}

// Azure/OpenAI style streaming structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureStreamChoice {
    #[serde(default)]
    pub index: u32,
    #[serde(default)]
    pub delta: Option<AzureStreamDelta>,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureStreamDelta {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub reasoning_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureStreamChunk {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub object: String,
    #[serde(default)]
    pub choices: Vec<AzureStreamChoice>,
}

#[derive(Debug)]
pub struct ClaudeClient {
    base_url: String,
    deployment_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_client_new() {
        let client = ClaudeClient::new(
            "https://api.anthropic.com".to_string(),
            Some("claude-3-opus".to_string()),
        );
        assert_eq!(client.deployment_name, "claude-3-opus");
    }

    #[test]
    fn test_claude_client_azure() {
        let client = ClaudeClient::azure(
            "https://myendpoint.openai.azure.com/anthropic".to_string(),
            "claude-opus-4-5".to_string(),
        );
        assert_eq!(client.deployment_name, "claude-opus-4-5");
    }

    #[test]
    fn test_build_messages_empty() {
        let (system, messages) = ClaudeClient::build_messages("", "", &[]);
        assert!(system.is_none());
        assert!(messages.is_empty());
    }

    #[test]
    fn test_build_messages_with_system() {
        let (system, messages) = ClaudeClient::build_messages("Be helpful", "", &[]);
        assert_eq!(system, Some("Be helpful".to_string()));
        assert!(messages.is_empty());
    }

    #[test]
    fn test_build_messages_with_history() {
        let history = vec![
            ("user".to_string(), "Hello".to_string()),
            ("assistant".to_string(), "Hi there".to_string()),
        ];
        let (system, messages) = ClaudeClient::build_messages("", "", &history);
        assert!(system.is_none());
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, "Hello");
    }
}

mod part1;
mod part2;
mod part3;
