use async_trait::async_trait;
use futures::StreamExt;
use log::{info, warn};
use serde_json::Value;
use tokio::sync::mpsc;

use crate::traits::LLMProvider;

#[derive(Debug)]
pub struct BedrockClient {
    client: reqwest::Client,
    base_url: String,
}

impl BedrockClient {
    pub fn new(base_url: String) -> Self {
        // Accept three URL formats:
        // 1. OpenAI-compatible: .../openai/v1/chat/completions (use as-is)
        // 2. Native invoke: .../model/{model-id}/invoke (use as-is, streaming swaps to invoke-with-response-stream)
        // 3. Bare domain: https://bedrock-runtime.region.amazonaws.com (auto-append OpenAI path)
        let url = if base_url.contains("/openai/") || base_url.contains("/chat/completions") || base_url.contains("/model/") {
            base_url
        } else {
            let trimmed = base_url.trim_end_matches('/');
            format!("{}/openai/v1/chat/completions", trimmed)
        };

        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(180))
                .connect_timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| {
                    log::warn!("bedrock: failed to build reqwest client, using default");
                    reqwest::Client::new()
                }),
            base_url: url,
        }
    }

    /// Check if URL is native Bedrock invoke endpoint (not OpenAI-compatible)
    fn is_native_invoke(&self) -> bool {
        self.base_url.contains("/model/") && self.base_url.contains("/invoke")
    }

    /// Get streaming URL: for native invoke, swap /invoke to /invoke-with-response-stream
    fn stream_url(&self) -> String {
        if self.is_native_invoke() && self.base_url.ends_with("/invoke") {
            self.base_url.replace("/invoke", "/invoke-with-response-stream")
        } else {
            self.base_url.clone()
        }
    }

    /// Build the auth header from the key
    fn auth_header(key: &str) -> String {
        if key.starts_with("Bearer ") {
            key.to_string()
        } else {
            format!("Bearer {}", key)
        }
    }

    /// Build formatted messages from raw input
    fn build_messages(raw_messages: &Value) -> Value {
        let mut messages_limited = Vec::new();
        if let Some(msg_array) = raw_messages.as_array() {
            for msg in msg_array {
                let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");
                let normalized_role = match role {
                    "user" | "assistant" | "system" | "developer" | "tool" => role,
                    "episodic" | "compact" => "system",
                    _ => "user",
                };
                let mut new_msg = msg.clone();
                if let Some(obj) = new_msg.as_object_mut() {
                    obj.insert("role".to_string(), serde_json::json!(normalized_role));
                }
                messages_limited.push(new_msg);
            }
        }
        Value::Array(messages_limited)
    }

    /// Send a streaming request and process the response
    async fn do_stream(
        &self,
        formatted_messages: &Value,
        model: &str,
        key: &str,
        tools: Option<&Vec<Value>>,
        tx: &mpsc::Sender<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let auth_header = Self::auth_header(key);

        let mut request_body = serde_json::json!({
            "model": model,
            "messages": formatted_messages,
            "stream": true
        });

        if let Some(tools_value) = tools {
            if !tools_value.is_empty() {
                request_body["tools"] = serde_json::json!(tools_value);
                info!("Added {} tools to Bedrock request", tools_value.len());
            }
        }

        let url = self.stream_url();
        info!("Sending streaming request to Bedrock endpoint: {}", url);

        let response = self.client
            .post(&url)
            .header("Authorization", &auth_header)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::error!("Bedrock generate_stream error: {}", error_text);
            return Err(format!("Bedrock API error ({}): {}", status, error_text).into());
        }

        let mut stream = response.bytes_stream();
        let mut tool_call_buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    if let Ok(text) = std::str::from_utf8(&chunk) {
                        for line in text.split('\n') {
                            let line = line.trim();
                            if let Some(data) = line.strip_prefix("data: ") {
                                if data == "[DONE]" {
                                    continue;
                                }

                                if let Ok(json) = serde_json::from_str::<Value>(data) {
                                    if let Some(choices) = json.get("choices") {
                                        if let Some(first_choice) = choices.get(0) {
                                            if let Some(delta) = first_choice.get("delta") {
                                                if let Some(content) = delta.get("content") {
                                                    if let Some(content_str) = content.as_str() {
                                                        if !content_str.is_empty() && tx.send(content_str.to_string()).await.is_err() {
                                                            return Ok(());
                                                        }
                                                    }
                                                }

                                                if let Some(tool_calls) = delta.get("tool_calls") {
                                                    if let Some(calls_array) = tool_calls.as_array() {
                                                        if let Some(first_call) = calls_array.first() {
                                                            if let Some(function) = first_call.get("function") {
                                                                if let Some(name) = function.get("name") {
                                                                    if let Some(name_str) = name.as_str() {
                                                                        tool_call_buffer = format!("{{\"name\": \"{}\", \"arguments\": \"", name_str);
                                                                    }
                                                                }
                                                                if let Some(args) = function.get("arguments") {
                                                                    if let Some(args_str) = args.as_str() {
                                                                        tool_call_buffer.push_str(args_str);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("Bedrock stream reading error: {}", e);
                    break;
                }
            }
        }

        if !tool_call_buffer.is_empty() {
            tool_call_buffer.push_str("\"}");
            let _ = tx.send(format!("`tool_call`: {}", tool_call_buffer)).await;
        }

        Ok(())
    }
}

#[async_trait]
impl LLMProvider for BedrockClient {
    async fn generate(
        &self,
        prompt: &str,
        messages: &Value,
        model: &str,
        key: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let default_messages = serde_json::json!([{"role": "user", "content": prompt}]);

        let raw_messages = if messages.is_array() && !messages.as_array().unwrap_or(&vec![]).is_empty() {
            messages
        } else {
            &default_messages
        };

        let formatted_messages = Self::build_messages(raw_messages);
        let auth_header = Self::auth_header(key);

        let request_body = serde_json::json!({
            "model": model,
            "messages": formatted_messages,
            "stream": false
        });

        info!("Sending request to Bedrock endpoint: {}", self.base_url);

        let response = self.client
            .post(&self.base_url)
            .header("Authorization", &auth_header)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::error!("Bedrock generate error: {}", error_text);
            return Err(format!("Bedrock API error ({}): {}", status, error_text).into());
        }

        let json: Value = response.json().await?;

        if let Some(choices) = json.get("choices") {
            if let Some(first_choice) = choices.get(0) {
                if let Some(message) = first_choice.get("message") {
                    if let Some(content) = message.get("content") {
                        if let Some(content_str) = content.as_str() {
                            return Ok(content_str.to_string());
                        }
                    }
                }
            }
        }

        Err("Failed to parse response from Bedrock".into())
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        messages: &Value,
        tx: mpsc::Sender<String>,
        model: &str,
        key: &str,
        tools: Option<&Vec<Value>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let default_messages = serde_json::json!([{"role": "user", "content": prompt}]);

        let raw_messages = if messages.is_array() && !messages.as_array().unwrap_or(&vec![]).is_empty() {
            messages
        } else {
            &default_messages
        };

        let formatted_messages = Self::build_messages(raw_messages);

        // Try with tools first
        let result = self.do_stream(&formatted_messages, model, key, tools, &tx).await;

        if let Err(ref e) = result {
            let err_str = e.to_string();
            // If error is "Operation not allowed" or validation_error, retry without tools
            if (err_str.contains("Operation not allowed") || err_str.contains("validation_error"))
                && tools.is_some()
            {
                warn!(
                    "Bedrock model '{}' does not support tools, retrying without tools",
                    model
                );
                return self.do_stream(&formatted_messages, model, key, None, &tx).await;
            }
        }

        result
    }

    async fn cancel_job(&self, _session_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}
