//! OpenAI/Azure chat-completion clients.
//! Split from lib.rs for file-size limits.

use async_trait::async_trait;

use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::{
    rate_limiter::ApiRateLimiter,
    provider_catalog::LLMProviderType,
    provider_resolve::{rate_limits_for_url, profiles_for_url, max_output_tokens_for_type},
    traits::LLMProvider,
};

pub struct OpenAIClient {
    pub(crate) client: reqwest::Client,
    pub(crate) base_url: String,
    pub(crate) endpoint_path: String,
    pub(crate) rate_limiter: Arc<ApiRateLimiter>,
    pub(crate) provider_type: LLMProviderType,
    pub(crate) max_output_tokens: u32,
}

#[derive(Debug)]
pub struct AzureGPT5Client {
    client: reqwest::Client,
    base_url: String,
    api_version: String,
    rate_limiter: Arc<ApiRateLimiter>,
}

impl AzureGPT5Client {
    pub fn new(base_url: String, api_version: Option<String>) -> Self {
        let api_version = api_version.unwrap_or_else(|| "2025-04-01-preview".to_string());
        let rate_limiter = Arc::new(ApiRateLimiter::unlimited());
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(180))
            .connect_timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url,
            api_version,
            rate_limiter,
        }
    }

    fn sanitize_utf8(input: &str) -> String {
        input.chars()
            .filter(|c| {
                let cp = *c as u32;
                !(0xD800..=0xDBFF).contains(&cp) && !(0xDC00..=0xDFFF).contains(&cp)
            })
            .collect()
    }
}

#[async_trait]
impl LLMProvider for AzureGPT5Client {
    async fn generate(
        &self,
        prompt: &str,
        config: &Value,
        model: &str,
        key: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let raw_messages = if config.is_array() && !config.as_array().unwrap_or(&vec![]).is_empty() {
            config
        } else {
            &serde_json::json!([{"role": "user", "content": prompt}])
        };

        if let Err(e) = self.rate_limiter.acquire(4096).await {
            log::error!("Rate limit exceeded: {}", e);
            return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>);
        }

        let full_url = format!(
            "{}/openai/responses?api-version={}",
            self.base_url, self.api_version
        );
        let auth_header = format!("Bearer {}", key);

        let input_array: Vec<Value> = raw_messages
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|msg| {
                serde_json::json!({
                    "role": msg.get("role").and_then(|r| r.as_str()).unwrap_or("user"),
                    "content": Self::sanitize_utf8(msg.get("content").and_then(|c| c.as_str()).unwrap_or(""))
                })
            })
            .collect();

        let response = self
            .client
            .post(&full_url)
            .header("Authorization", &auth_header)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": model,
                "input": input_array,
                "max_output_tokens": 16384
            }))
            .send()
            .await?;

        let status = response.status();
        if status != reqwest::StatusCode::OK {
            let error_text = response.text().await.unwrap_or_default();
            log::error!("AzureGPT5 generate error: {}", error_text);
            return Err(format!("AzureGPT5 request failed with status: {}", status).into());
        }

        let result: Value = response.json().await?;
        let content = result["output"][0]["content"][0]["text"]
            .as_str()
            .unwrap_or("");

        Ok(content.to_string())
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        config: &Value,
        tx: mpsc::Sender<String>,
        model: &str,
        key: &str,
        _tools: Option<&Vec<Value>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let content = self.generate(prompt, config, model, key).await?;
        tx.send(content).await?;
        Ok(())
    }

    async fn cancel_job(
        &self,
        _session_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}

impl std::fmt::Debug for OpenAIClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAIClient")
            .field("base_url", &self.base_url)
            .field("endpoint_path", &self.endpoint_path)
            .finish()
    }
}

impl OpenAIClient {
    pub fn estimate_tokens(text: &str) -> usize {
        text.len().div_ceil(4)
    }

    pub fn estimate_messages_tokens(messages: &Value) -> usize {
        if let Some(msg_array) = messages.as_array() {
            msg_array
                .iter()
                .map(|msg| {
                    if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                        Self::estimate_tokens(content)
                    } else {
                        0
                    }
                })
                .sum()
        } else {
            0
        }
    }

    pub fn truncate_messages(messages: &Value, max_tokens: usize) -> Value {
        let mut result = Vec::new();
        let mut token_count = 0;

        if let Some(msg_array) = messages.as_array() {
            for msg in msg_array {
                if let Some(role) = msg.get("role").and_then(|r| r.as_str()) {
                    if role == "system" {
                        if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                            let msg_tokens = Self::estimate_tokens(content);
                            if token_count + msg_tokens <= max_tokens {
                                result.push(msg.clone());
                                token_count += msg_tokens;
                            }
                        }
                    }
                }
            }

            let mut recent_messages: Vec<&Value> = msg_array
                .iter()
                .filter(|msg| msg.get("role").and_then(|r| r.as_str()) != Some("system"))
                .collect();

            recent_messages.reverse();

            for msg in recent_messages {
                if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                    let msg_tokens = Self::estimate_tokens(content);
                    if token_count + msg_tokens <= max_tokens {
                        result.push(msg.clone());
                        token_count += msg_tokens;
                    } else {
                        break;
                    }
                }
            }

            let system_count = result.len()
                - result
                    .iter()
                    .filter(|m| m.get("role").and_then(|r| r.as_str()) != Some("system"))
                    .count();
            let mut user_messages: Vec<Value> = result.drain(system_count..).collect();
            user_messages.reverse();
            result.extend(user_messages);
        }

        serde_json::Value::Array(result)
    }

    pub fn ensure_token_limit(messages: &Value, model_context_limit: usize) -> Value {
        let estimated_tokens = Self::estimate_messages_tokens(messages);
        let safe_limit = (model_context_limit as f64 * 0.9) as usize;

        if estimated_tokens > safe_limit {
            log::warn!(
                "Messages exceed token limit ({} > {}), truncating...",
                estimated_tokens,
                safe_limit
            );
            Self::truncate_messages(messages, safe_limit)
        } else {
            messages.clone()
        }
    }

    pub fn new(_api_key: String, base_url: Option<String>, endpoint_path: Option<String>) -> Self {
        let base = base_url.unwrap_or_else(|| "https://api.openai.com".to_string());
        // A caller that did not name a provider still gets the right limits and
        // token ceiling: the catalog recognises the host from its URL.
        let provider_type = LLMProviderType::from(base.as_str());
        Self::with_provider(base, endpoint_path, provider_type)
    }

    /// Builds a client for a named member of the OpenAI-compatible tier.
    ///
    /// `provider_type` is the authoritative identity — it comes from the
    /// `llm-provider` bot-config value when the operator set one — and selects
    /// the operating caps and the `max_tokens` ceiling from the catalog. The
    /// URL is only consulted for hosts the operator left unnamed.
    pub fn with_provider(
        base: String,
        endpoint_path: Option<String>,
        provider_type: LLMProviderType,
    ) -> Self {
        let trimmed_base = base.trim_end_matches('/').to_string();

        let has_v1_path = trimmed_base.contains("/v1/chat/completions");
        let has_chat_path = !has_v1_path && trimmed_base.contains("/chat/completions");

        let endpoint = if let Some(path) = endpoint_path {
            path
        } else if has_v1_path || (has_chat_path && !trimmed_base.contains("z.ai")) {
            "".to_string()
        } else if trimmed_base.contains("z.ai") || trimmed_base.contains("/v4") {
            "/chat/completions".to_string()
        } else {
            "/v1/chat/completions".to_string()
        };

        let (final_base, final_endpoint) = if !endpoint.is_empty() && trimmed_base.ends_with(&endpoint) {
            (trimmed_base, "".to_string())
        } else {
            (trimmed_base, endpoint)
        };

        // Limits and the token ceiling belong to the *host*: `groq.com` and
        // `api.openai.com` share the generic `openai` provider identity but not
        // the same quota, so the URL is consulted first and the operator's named
        // provider is the fallback for a URL the catalog does not recognise.
        let rate_limits = rate_limits_for_url(&final_base);
        let max_output_tokens = profiles_for_url(&final_base)
            .first()
            .map(|p| p.max_output_tokens)
            .unwrap_or_else(|| max_output_tokens_for_type(provider_type));
        let rate_limiter = ApiRateLimiter::new(rate_limits);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(180))
            .connect_timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| {
                log::warn!("Failed to build reqwest client with timeouts, falling back to default");
                reqwest::Client::new()
            });

        Self {
            client,
            base_url: final_base,
            endpoint_path: final_endpoint,
            rate_limiter: Arc::new(rate_limiter),
            provider_type,
            max_output_tokens,
        }
    }

    /// The catalog row backing this client's limits and token ceiling, or the
    /// generic default when the host is unrecognised.
    #[must_use]
    pub fn provider_type(&self) -> LLMProviderType {
        self.provider_type
    }

    pub fn sanitize_utf8(input: &str) -> String {
        input.chars()
            .filter(|c| {
                let cp = *c as u32;
                !(0xD800..=0xDBFF).contains(&cp) && !(0xDC00..=0xDFFF).contains(&cp)
            })
            .collect()
    }

    pub fn build_messages(
        system_prompt: &str,
        context_data: &str,
        history: &[(String, String)],
    ) -> Value {
        let mut messages = Vec::new();
        if !system_prompt.is_empty() {
            messages.push(serde_json::json!({
                "role": "system",
                "content": Self::sanitize_utf8(system_prompt)
            }));
        }
        if !context_data.is_empty() {
            messages.push(serde_json::json!({
                "role": "system",
                "content": Self::sanitize_utf8(context_data)
            }));
        }
        for (role, content) in history {
            let api_role = match role.as_str() {
                "user" | "assistant" | "system" | "developer" | "tool" => role.as_str(),
                "episodic" | "compact" => "system",
                _ => "system",
            };
            messages.push(serde_json::json!({
                "role": api_role,
                "content": Self::sanitize_utf8(content)
            }));
        }
        serde_json::Value::Array(messages)
    }
}

