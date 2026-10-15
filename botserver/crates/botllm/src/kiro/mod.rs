//! Kiro (kiro.dev) LLM provider.
//!
//! Kiro API keys (`ksk_…`) authenticate against the Amazon Q Developer /
//! CodeWhisperer backend — NOT an OpenAI-compatible endpoint. The protocol is
//! AWS JSON 1.0 over `X-Amz-Target:
//! AmazonCodeWhispererStreamingService.GenerateAssistantResponse` at
//! `https://q.<region>.amazonaws.com/`, and the response is an AWS Event
//! Stream binary envelope with JSON event objects embedded (`{"content":…}`,
//! `{"toolUseId":…}`, `{"usage":…}`, …).
//!
//! Reference protocol behavior (pi-kiro / kiro-gateway, MIT):
//! - `Authorization: Bearer <ksk key>` + `tokentype: API_KEY` header
//! - `origin: AI_EDITOR` on every user message; `agentTaskType: "vibe"`
//! - alternating `userInputMessage` / `assistantResponseMessage` history
//! - `INSUFFICIENT_MODEL_CAPACITY` errors are retryable with backoff

pub(crate) use crate::llm_models::{get_handler, ModelHandler};
pub(crate) use crate::traits::LLMProvider;
pub(crate) use async_trait::async_trait;
pub(crate) use log::{error, warn};
pub(crate) use serde_json::{json, Value};
pub(crate) use std::error::Error;
pub(crate) use std::sync::Arc;
pub(crate) use std::time::Duration;
pub(crate) use tokio::sync::mpsc;

const DEFAULT_ENDPOINT: &str = "https://q.us-east-1.amazonaws.com/";
const TARGET_STREAMING: &str = "AmazonCodeWhispererStreamingService.GenerateAssistantResponse";
const ORIGIN: &str = "AI_EDITOR";
const MAX_CAPACITY_RETRIES: u32 = 3;
/// Aligns with the other providers (OpenAI/GLM/Bedrock use 180s).
const REQUEST_TIMEOUT_SECS: u64 = 180;

impl KiroClient {
    /// `true` when a configured LLM url points at the Kiro / CodeWhisperer
    /// endpoint rather than an OpenAI-compatible API (#1498).
    pub fn is_kiro_endpoint(url: &str) -> bool {
        let u = url.to_lowercase();
        u.contains("kiro")
            || u.contains("q.us-east-1.amazonaws.com")
            || u.contains("codewhisperer")
            || u.contains("q.eu-central-1.amazonaws.com")
    }
}

#[derive(Debug)]
pub struct KiroClient {
    http: reqwest::Client,
    endpoint: String,
    rate_limiter: Arc<crate::rate_limiter::ApiRateLimiter>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // #1390 — the kiro.dev key is currently NOT entitled (403
    // "User is not authorized to make this call"), so error classification
    // must distinguish it from an invalid key. When the account becomes
    // entitled these assertions pin the contract.
    #[test]
    fn capacity_error_is_classified() {
        assert!(KiroClient::is_capacity_error("{\"error\":\"INSUFFICIENT_MODEL_CAPACITY\"}"));
        assert!(!KiroClient::is_capacity_error("{\"error\":\"AccessDeniedException\"}"));
    }

    #[test]
    fn tools_are_advertised_as_specifications() {
        let tools = vec![json!({
            "type": "function",
            "function": {
                "name": "file/write",
                "description": "Write a file",
                "parameters": {"type": "object", "properties": {"path": {"type": "string"}}}
            }
        })];
        let messages = json!([
            {"role": "system", "content": "SYS"},
            {"role": "user", "content": "do it"},
        ]);
        let body = KiroClient::build_request(&messages, "claude-sonnet-4-5", &tools);
        let uim = &body["conversationState"]["currentMessage"]["userInputMessage"];
        assert_eq!(uim["modelId"], json!("claude-sonnet-4-5"));
        assert_eq!(uim["origin"], json!("AI_EDITOR"));
        assert!(uim["content"].as_str().unwrap_or("").starts_with("SYS\n\ndo it"));
        let spec = &uim["userInputMessageContext"]["tools"][0]["toolSpecification"];
        assert_eq!(spec["name"], json!("file/write"));
        assert_eq!(spec["inputSchema"]["json"]["properties"]["path"]["type"], json!("string"));
    }

    #[test]
    fn endpoints_are_detected() {
        assert!(KiroClient::is_kiro_endpoint("https://q.us-east-1.amazonaws.com/"));
        assert!(KiroClient::is_kiro_endpoint("kiro"));
        assert!(!KiroClient::is_kiro_endpoint("https://integrate.api.nvidia.com/v1/chat/completions"));
    }

    #[test]
    fn model_ids_are_normalized() {
        assert_eq!(KiroClient::kiro_model_id("claude-sonnet-4-5"), "claude-sonnet-4-5");
        assert_eq!(KiroClient::kiro_model_id("Claude-Sonnet-4.5"), "claude-sonnet-4-5");
        assert_eq!(KiroClient::kiro_model_id("claude-sonnet-4-5-20250929"), "claude-sonnet-4-5");
        assert_eq!(KiroClient::kiro_model_id("glm-5"), "GLM-5");
        assert_eq!(KiroClient::kiro_model_id("unknown-model"), "auto");
    }

    #[test]
    fn entitle_error_message_is_surfaced() {
        let body = r#"{"__type":"com.amazon.aws.codewhisperer#AccessDeniedException","message":"User is not authorized to make this call."}"#;
        let msg = KiroClient::extract_error_text(body);
        assert!(msg.contains("not authorized"), "got: {msg}");
    }

    #[test]
    fn invalid_key_error_is_distinct_from_entitlement() {
        // Bogus key: "The bearer token included in the request is invalid."
        // Entitled-but-unauthorized key: "User is not authorized to make this call."
        let invalid = KiroClient::extract_error_text(r#"{"message":"The bearer token included in the request is invalid."}"#);
        let unentitled = KiroClient::extract_error_text(r#"{"message":"User is not authorized to make this call."}"#);
        assert!(invalid.contains("bearer token"));
        assert!(unentitled.contains("not authorized"));
        assert_ne!(invalid, unentitled);
    }
}

mod part1;
mod part2;
mod part3;

use part3::*;
