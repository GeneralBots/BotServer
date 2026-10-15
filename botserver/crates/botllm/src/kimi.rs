use async_trait::async_trait;
use futures::StreamExt;
use log::info;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;

use super::traits::LLMProvider;

// Kimi K2.5 (moonshotai) API Client
// NVIDIA endpoint with special chat_template_kwargs for thinking mode

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KimiMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<Vec<Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KimiRequest {
    pub model: String,
    pub messages: Vec<KimiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<Value>,
    #[serde(rename = "chat_template_kwargs", skip_serializing_if = "Option::is_none")]
    pub chat_template_kwargs: Option<KimiChatTemplateKwargs>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KimiChatTemplateKwargs {
    pub thinking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KimiResponseChoice {
    #[serde(default)]
    pub index: u32,
    pub message: KimiMessage,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KimiResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<KimiResponseChoice>,
    #[serde(default)]
    pub usage: Option<Value>,
}

// Streaming structures
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KimiStreamDelta {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<Vec<Value>>,
    #[serde(default)]
    pub reasoning_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KimiStreamChoice {
    #[serde(default)]
    pub index: u32,
    #[serde(default)]
    pub delta: KimiStreamDelta,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KimiStreamChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<KimiStreamChoice>,
    #[serde(default)]
    pub usage: Option<Value>,
}

#[derive(Debug)]
pub struct KimiClient {
    client: reqwest::Client,
    base_url: String,
}

impl KimiClient {
    pub fn new(base_url: String) -> Self {
        let base = base_url.trim_end_matches('/').to_string();

        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(180))
                .connect_timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| {
                    log::warn!("kimi: failed to build reqwest client, using default");
                    reqwest::Client::new()
                }),
            base_url: base,
        }
    }

    fn build_url(&self) -> String {
        if self.base_url.contains("/chat/completions") {
            self.base_url.clone()
        } else {
            format!("{}/chat/completions", self.base_url)
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
impl LLMProvider for KimiClient {
    async fn generate(
        &self,
        prompt: &str,
        _config: &Value,
        model: &str,
        key: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let messages = vec![KimiMessage {
            role: "user".to_string(),
            content: Some(prompt.to_string()),
            tool_calls: None,
        }];

        let model_name = if model == "kimi-k2.5" || model == "kimi-k2" {
            "moonshotai/kimi-k2.5"
        } else {
            model
        };

        let request = KimiRequest {
            model: model_name.to_string(),
            messages,
            stream: Some(false),
            max_tokens: Some(131072),
            temperature: Some(1.0),
            top_p: Some(1.0),
            tools: None,
            tool_choice: None,
            chat_template_kwargs: Some(KimiChatTemplateKwargs {
                thinking: true,
            }),
        };

        let url = self.build_url();
        info!("Kimi non-streaming request to: {}", url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", key))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::error!("Kimi API error: {}", error_text);
            return Err(format!("Kimi API error: {}", error_text).into());
        }

        let kimi_response: KimiResponse = response.json().await?;
        let content = kimi_response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(content)
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        config: &Value,
        tx: mpsc::Sender<String>,
        model: &str,
        key: &str,
        tools: Option<&Vec<Value>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let messages = if let Some(msgs) = config.as_array() {
            msgs.iter()
                .filter_map(|m| {
                    let role = m.get("role")?.as_str()?;
                    let content = m.get("content")?.as_str()?;
                    let sanitized = Self::sanitize_utf8(content);
                    let normalized_role = match role {
                        "user" | "assistant" | "system" | "tool" => role,
                        "episodic" | "compact" => "system",
                        _ => "user",
                    };
                    Some(KimiMessage {
                        role: normalized_role.to_string(),
                        content: Some(sanitized),
                        tool_calls: None,
                    })
                })
                .collect::<Vec<_>>()
        } else {
            vec![KimiMessage {
                role: "user".to_string(),
                content: Some(Self::sanitize_utf8(prompt)),
                tool_calls: None,
            }]
        };

        if messages.is_empty() {
            return Err("No valid messages in request".into());
        }

        let model_name = if model == "kimi-k2.5" || model == "kimi-k2" {
            "moonshotai/kimi-k2.5"
        } else {
            model
        };

        let tool_choice = if tools.is_some() {
            Some(serde_json::json!("auto"))
        } else {
            None
        };

        let request = KimiRequest {
            model: model_name.to_string(),
            messages,
            stream: Some(true),
            max_tokens: Some(131072),
            temperature: Some(1.0),
            top_p: Some(1.0),
            tools: tools.cloned(),
            tool_choice,
            chat_template_kwargs: Some(KimiChatTemplateKwargs {
                thinking: true,
            }),
        };

        let url = self.build_url();
        info!("[Kimi] Streaming request to: {} model={} max_tokens=131072", url, model_name);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", key))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::error!("[Kimi] Streaming error: {}", error_text);
            return Err(format!("Kimi streaming error: {}", error_text).into());
        }

        info!("[Kimi] Connection established, starting stream");

        let handler = crate::llm_models::get_handler(model);
        let mut stream_state = String::new();
        let mut stream = response.bytes_stream();
        let mut total_content_chars: usize = 0;
        let mut chunk_count: usize = 0;
        let mut buffer = Vec::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| format!("Stream error: {}", e))?;

            buffer.extend_from_slice(&chunk);
            let data = String::from_utf8_lossy(&buffer);

            for line in data.lines() {
                let line = line.trim();

                if line.is_empty() {
                    continue;
                }

                if line == "data: [DONE]" {
                    info!("[Kimi] Stream done: {} chunks, {} content chars sent", chunk_count, total_content_chars);
                    let _ = tx.send(String::new()).await;
                    return Ok(());
                }

                if let Some(json_str) = line.strip_prefix("data: ") {
                    let json_str = json_str.trim();
                    if let Ok(chunk_data) = serde_json::from_str::<Value>(json_str) {
                        if let Some(choices) = chunk_data.get("choices").and_then(|c| c.as_array()) {
                            for choice in choices {
                                if let Some(delta) = choice.get("delta") {
                                    chunk_count += 1;

                                    if let Some(tool_calls) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                                        for tool_call in tool_calls {
                                            let tool_call_json = serde_json::json!({
                                                "type": "tool_call",
                                                "content": tool_call
                                            }).to_string();
                                            let _ = tx.send(tool_call_json).await;
                                        }
                                    }

              // Kimi K2.5: content has the answer, reasoning/reasoning_content is thinking
              if let Some(text) = delta.get("content").and_then(|c| c.as_str()) {
                if !text.is_empty() {
                  let processed = handler.process_content_streaming(text, &mut stream_state);
                  if !processed.content.is_empty() {
                    total_content_chars += processed.content.len();
                    if tx.send(processed.content).await.is_err() {
                      info!("[Kimi] Channel closed, stopping stream after {} content chars", total_content_chars);
                      return Ok(());
                    }
                  }
                  if !processed.reasoning.is_empty() {
                    let reasoning_msg = serde_json::json!({
                        "__reasoning__": processed.reasoning
                    }).to_string();
                    let _ = tx.send(reasoning_msg).await;
                  }
                }
              }

              // Check for content filter errors
              if let Some(filter_result) = delta.get("content_filter_result") {
                if let Some(error) = filter_result.get("error") {
                  let code = error.get("code").and_then(|c| c.as_str()).unwrap_or("unknown");
                  let message = error.get("message").and_then(|m| m.as_str()).unwrap_or("no message");
                  log::error!("[Kimi] Content filter error: code={}, message={}", code, message);
                } else {
                  log::trace!("[Kimi] Content filter result (no error): {:?}", filter_result);
                }
              }
            }

                                if let Some(reason) = choice.get("finish_reason").and_then(|r| r.as_str()) {
                                    if !reason.is_empty() {
                                        info!("[Kimi] Stream finished: {}, {} content chars", reason, total_content_chars);
                                        let _ = tx.send(String::new()).await;
                                        return Ok(());
                                    }
                                }
                            }
                        }
                    } else {
                        log::trace!("[Kimi] Failed to parse JSON: {} chars", json_str.len());
                    }
                }
            }

            // Keep only unprocessed data in buffer
            if let Some(last_newline) = data.rfind('\n') {
                buffer = buffer[last_newline + 1..].to_vec();
            }
        }

        info!("[Kimi] Stream ended (no [DONE]), {} chunks, {} content chars", chunk_count, total_content_chars);
        let _ = tx.send(String::new()).await;
        Ok(())
    }

    async fn cancel_job(
        &self,
        _session_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Kimi cancel requested for session {} (no-op)", _session_id);
        Ok(())
    }
}
