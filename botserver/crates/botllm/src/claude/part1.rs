use super::*;

impl ClaudeClient {
    pub fn new(base_url: String, deployment_name: Option<String>) -> Self {
        Self {
            base_url,
            deployment_name: deployment_name.unwrap_or_else(|| "claude-opus-4-5".to_string()),
        }
    }

    pub fn azure(endpoint: String, deployment_name: String) -> Self {
        Self {
            base_url: endpoint,
            deployment_name,
        }
    }

    pub(super) fn get_retry_delay(attempt: u32) -> Duration {
        let delay = (INITIAL_DELAY_MS as f64 * BACKOFF_MULTIPLIER.powi(attempt as i32))
            .min(MAX_DELAY_MS as f64);
        // Add jitter like Node.js: delay * 0.25 * (Math.random() * 2 - 1)
        let jitter = delay * 0.25 * (rand::random::<f64>() * 2.0 - 1.0);
        Duration::from_millis((delay + jitter) as u64)
    }

    pub(super) fn is_retryable_error(err_msg: &str, status_code: Option<u16>) -> bool {
        // Retryable status codes matching Node.js: [408, 429, 500, 502, 503, 504]
        if let Some(code) = status_code {
            if [408, 429, 500, 502, 503, 504].contains(&code) {
                return true;
            }
        }

        // Retryable error patterns matching Node.js
        let msg = err_msg.to_lowercase();
        msg.contains("timeout")
            || msg.contains("econnreset")
            || msg.contains("etimedout")
            || msg.contains("enotfound")
            || msg.contains("econnrefused")
            || msg.contains("epipe")
            || msg.contains("ehostunreach")
            || msg.contains("aborted")
            || msg.contains("socket hang up")
            || msg.contains("network")
            || msg.contains("connection reset")
            || msg.contains("broken pipe")
            || msg.contains("connection closed")
    }

    pub(super) fn build_messages_from_value(
        &self,
        prompt: &str,
        messages: &Value,
    ) -> (Option<String>, Vec<ClaudeMessage>) {
        let empty_vec = vec![];

        let mut claude_messages: Vec<ClaudeMessage> = if messages.is_array() {
            let arr = messages.as_array().unwrap_or(&empty_vec);
            if arr.is_empty() {
                vec![ClaudeMessage {
                    role: "user".to_string(),
                    content: prompt.to_string(),
                }]
            } else {
                arr.iter()
                    .filter_map(|m| {
                        let role = m["role"].as_str().unwrap_or("user");
                        let content = m["content"].as_str().unwrap_or("");
                        if role == "system"
                            || role == "episodic"
                            || role == "compact"
                            || content.is_empty()
                        {
                            None
                        } else {
                            let normalized_role = match role {
                                "user" | "assistant" => role.to_string(),
                                _ => "user".to_string(),
                            };
                            Some(ClaudeMessage {
                                role: normalized_role,
                                content: content.to_string(),
                            })
                        }
                    })
                    .collect()
            }
        } else {
            vec![ClaudeMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }]
        };

        if claude_messages.is_empty() && !prompt.is_empty() {
            claude_messages.push(ClaudeMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            });
        }

        // Extract system messages
        let system_prompt: Option<String> = if messages.is_array() {
            let system_text: String = messages
                .as_array()
                .unwrap_or(&empty_vec)
                .iter()
                .filter(|m| m["role"].as_str() == Some("system"))
                .map(|m| m["content"].as_str().unwrap_or("").to_string())
                .collect::<Vec<_>>()
                .join("\n\n");
            if system_text.is_empty() {
                None
            } else {
                Some(system_text)
            }
        } else {
            None
        };

        (system_prompt, claude_messages)
    }

    /// Sanitizes a string by removing invalid UTF-8 surrogate characters
    pub(super) fn sanitize_utf8(input: &str) -> String {
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
    ) -> (Option<String>, Vec<ClaudeMessage>) {
        let mut messages = Vec::new();
        let mut system_parts = Vec::new();

        if !system_prompt.is_empty() {
            system_parts.push(Self::sanitize_utf8(system_prompt));
        }
        if !context_data.is_empty() {
            system_parts.push(Self::sanitize_utf8(context_data));
        }

        for (role, content) in history {
            if role == "episodic" || role == "compact" {
                system_parts.push(format!("[Previous conversation summary]: {}", Self::sanitize_utf8(content)));
            }
        }

        let system = if system_parts.is_empty() {
            None
        } else {
            Some(system_parts.join("\n\n"))
        };

        let mut last_role: Option<String> = None;

        for (role, content) in history {
            let normalized_role = match role.as_str() {
                "user" => Some("user".to_string()),
                "assistant" => Some("assistant".to_string()),
                "system" | "episodic" | "compact" => None,
                _ => Some("user".to_string()),
            };

            if let Some(norm_role) = normalized_role {
                let sanitized_content = Self::sanitize_utf8(content);
                if sanitized_content.is_empty() {
                    continue;
                }

                if Some(&norm_role) == last_role.as_ref() {
                    if let Some(last_msg) = messages.last_mut() {
                        let last_msg: &mut ClaudeMessage = last_msg;
                        last_msg.content.push_str("\n\n");
                        last_msg.content.push_str(&sanitized_content);
                        continue;
                    }
                }

                messages.push(ClaudeMessage {
                    role: norm_role.clone(),
                    content: sanitized_content,
                });
                last_role = Some(norm_role);
            }
        }

        (system, messages)
    }

    pub(super) fn extract_text_from_response(&self, response: &ClaudeResponse) -> String {
        response
            .content
            .iter()
            .filter(|block| block.content_type == "text")
            .map(|block| block.text.clone())
            .collect::<Vec<_>>()
            .join("")
    }

    /// Process SSE data and extract content/reasoning
    pub(super) fn process_sse_data(&self, data: &str, model_name: &str) -> (Option<String>, Option<String>) {
        if data == "[DONE]" {
            return (None, None);
        }

        let handler = get_handler(model_name);

        // Try Azure/OpenAI format first (chat.completion.chunk)
        if let Ok(chunk) = serde_json::from_str::<AzureStreamChunk>(data) {
            if chunk.object == "chat.completion.chunk" || !chunk.choices.is_empty() {
                for choice in &chunk.choices {
                    if let Some(delta) = &choice.delta {
                        // Check for reasoning content (O1/DeepSeek/Claude on Azure style)
                        if let Some(reasoning) = &delta.reasoning_content {
                            if !reasoning.is_empty() {
                                return (None, Some(reasoning.to_string()));
                            }
                        }

                        // Get content
                        if let Some(text) = &delta.content {
                            if !text.is_empty() {
                                let processed = handler.process_content(text);
                                if !processed.is_empty() {
                                    return (Some(processed), None);
                                }
                            }
                        }
                    }
                }
                return (None, None);
            }
        }

        // Try standard Anthropic SSE format
        if let Ok(event) = serde_json::from_str::<ClaudeStreamEvent>(data) {
            if event.event_type.as_str() == "content_block_delta" {
                if let Some(delta) = event.delta {
                    if delta.delta_type == "text_delta" && !delta.text.is_empty() {
                        let processed = handler.process_content(&delta.text);
                        if !processed.is_empty() {
                            return (Some(processed), None);
                        }
                    } else if delta.delta_type == "thinking_delta" && !delta.thinking.is_empty() {
                        return (None, Some(delta.thinking.clone()));
                    }
                }
            }
        }

        (None, None)
    }
}
