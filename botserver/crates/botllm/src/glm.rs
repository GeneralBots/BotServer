use async_trait::async_trait;
use futures::StreamExt;
use log::{info, trace};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;

use super::traits::LLMProvider;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GLMMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Option<Vec<Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GLMChatTemplateKwargs {
    pub enable_thinking: bool,
    pub clear_thinking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GLMRequest {
    pub model: String,
    pub messages: Vec<GLMMessage>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_template_kwargs: Option<GLMChatTemplateKwargs>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GLMResponseChoice {
    #[serde(default)]
    pub index: u32,
    pub message: GLMMessage,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GLMResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<GLMResponseChoice>,
    #[serde(default)]
    pub usage: Option<Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GLMStreamDelta {
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
pub struct GLMStreamChoice {
    #[serde(default)]
    pub index: u32,
    pub delta: GLMStreamDelta,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GLMStreamChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<GLMStreamChoice>,
    #[serde(default)]
    pub usage: Option<Value>,
}

#[derive(Debug)]
pub struct GLMClient {
    client: reqwest::Client,
    base_url: String,
}

impl GLMClient {
    pub fn new(base_url: String) -> Self {
        let base = base_url.trim_end_matches('/').to_string();

        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(180))
                .connect_timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| {
                    log::warn!("glm: failed to build reqwest client, using default");
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
impl LLMProvider for GLMClient {
    async fn generate(
        &self,
        prompt: &str,
        _config: &Value,
        model: &str,
        key: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let messages = vec![GLMMessage {
            role: "user".to_string(),
            content: Some(prompt.to_string()),
            tool_calls: None,
        }];

        let model_name = if model == "glm-4" || model == "glm-4.7" {
            "z-ai/glm4.7"
        } else {
            model
        };

        let request = GLMRequest {
            model: model_name.to_string(),
            messages,
            stream: Some(false),
            max_tokens: Some(131072),
            temperature: Some(1.0),
            top_p: Some(1.0),
            tools: None,
            tool_choice: None,
            chat_template_kwargs: Some(GLMChatTemplateKwargs {
                enable_thinking: true,
                clear_thinking: false,
            }),
        };

        let url = self.build_url();
        info!("[GLM] Non-streaming request to: {} model={}", url, model_name);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::error!("[GLM] API error: {}", error_text);
            return Err(format!("GLM API error: {}", error_text).into());
        }

        let glm_response: GLMResponse = response.json().await?;
        let content = glm_response
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
                    Some(GLMMessage {
                        role: normalized_role.to_string(),
                        content: Some(sanitized),
                        tool_calls: None,
                    })
                })
                .collect::<Vec<_>>()
        } else {
            vec![GLMMessage {
                role: "user".to_string(),
                content: Some(Self::sanitize_utf8(prompt)),
                tool_calls: None,
            }]
        };

        if messages.is_empty() {
            return Err("No valid messages in request".into());
        }

        let model_name = if model == "glm-4" || model == "glm-4.7" {
            "z-ai/glm4.7"
        } else {
            model
        };

        let tool_choice = if tools.is_some() {
            Some(serde_json::json!("auto"))
        } else {
            None
        };

        let request = GLMRequest {
            model: model_name.to_string(),
            messages,
            stream: Some(true),
            max_tokens: Some(131072),
            temperature: Some(1.0),
            top_p: Some(1.0),
            tools: tools.cloned(),
            tool_choice,
chat_template_kwargs: Some(GLMChatTemplateKwargs {
                enable_thinking: true,
                clear_thinking: false,
            }),
        };

        let url = self.build_url();
        info!("[GLM] Streaming request to: {} model={} max_tokens=131072", url, model_name);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::error!("[GLM] Streaming error: {}", error_text);
            return Err(format!("GLM streaming error: {}", error_text).into());
        }

        info!("[GLM] Connection established, starting stream processing");

        let mut stream = response.bytes_stream();
        let mut in_reasoning = false;
        let mut has_sent_thinking = false;
        let mut total_content_chars: usize = 0;
        let mut total_reasoning_chars: usize = 0;
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
                    info!("[GLM] Stream done: {} chunks, {} reasoning chars, {} content chars sent", chunk_count, total_reasoning_chars, total_content_chars);
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

                                    // Handle tool_calls
                                    if let Some(tool_calls) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                                        for tool_call in tool_calls {
                                            let tool_call_json = serde_json::json!({
                                                "type": "tool_call",
                                                "content": tool_call
                                            }).to_string();
                                            let _ = tx.send(tool_call_json).await;
                                        }
                                    }

                                    // Handle reasoning_content (thinking phase)
                                    let reasoning = delta.get("reasoning_content")
                                        .and_then(|r| r.as_str())
                                        .or_else(|| delta.get("reasoning").and_then(|r| r.as_str()));

                                    let content = delta.get("content").and_then(|c| c.as_str());

                                    // Enter reasoning mode
                                    if reasoning.is_some() && content.is_none() {
                                        if !in_reasoning {
                                            info!("[GLM] Entering reasoning mode");
                                            in_reasoning = true;
                                        }
                                        if let Some(r) = reasoning {
                                            total_reasoning_chars += r.len();
                                        }
                                        if !has_sent_thinking {
                                            let thinking = serde_json::json!({
                                                "type": "thinking",
                                                "content": "🤔 Pensando..."
                                            }).to_string();
                                            let _ = tx.send(thinking).await;
                                            has_sent_thinking = true;
                                        }
                                        continue;
                                    }

                                    // Exited reasoning — content is now real response
                                    if in_reasoning && content.is_some() {
                                        info!("[GLM] Exited reasoning mode, {} reasoning chars discarded, content starting", total_reasoning_chars);
                                        in_reasoning = false;
                                        let clear = serde_json::json!({
                                            "type": "thinking_clear",
                                            "content": ""
                                        }).to_string();
                                        let _ = tx.send(clear).await;
                                    }

                                    // Send actual content to user
                                    if let Some(text) = content {
                                        if !text.is_empty() {
                                            total_content_chars += text.len();
                                            let _ = tx.send(text.to_string()).await;
                                        }
                                    }
                                } else {
                                    // No delta in choice
                                    trace!("[GLM] Chunk has no delta");
                                }

                                if let Some(reason) = choice.get("finish_reason").and_then(|r| r.as_str()) {
                                    if !reason.is_empty() {
                                        info!("[GLM] Stream finished: {}, reasoning={} content={}", reason, total_reasoning_chars, total_content_chars);
                                        let _ = tx.send(String::new()).await;
                                        return Ok(());
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Keep unprocessed data in buffer
            if let Some(last_newline) = data.rfind('\n') {
                buffer = buffer[last_newline + 1..].to_vec();
            }
        }

        info!("[GLM] Stream ended (no [DONE]), reasoning={} content={}", total_reasoning_chars, total_content_chars);
        let _ = tx.send(String::new()).await;
        Ok(())
    }

    async fn cancel_job(
        &self,
        _session_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("[GLM] Cancel requested for session {} (no-op)", _session_id);
        Ok(())
    }
}