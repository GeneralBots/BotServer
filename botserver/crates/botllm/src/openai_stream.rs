//! OpenAI streaming implementation.
//! Split from openai_client.rs for file-size limits.

use async_trait::async_trait;
use log::{info, trace, warn};
use serde_json::Value;
use tokio::sync::mpsc;

use crate::openai_client::OpenAIClient;
use crate::provider_catalog::LLMProviderType;

use crate::retry;
use crate::LLMProvider;

#[async_trait]
impl LLMProvider for OpenAIClient {
    async fn generate_stream(
        &self,
        prompt: &str,
        messages: &Value,
        tx: mpsc::Sender<String>,
        model: &str,
        key: &str,
        tools: Option<&Vec<Value>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        log::info!("OpenAIClient::generate_stream ENTERED: model={}, key_len={}, url={}{}",
            model, key.len(), self.base_url, self.endpoint_path);
        let default_messages = serde_json::json!([{"role": "user", "content": prompt}]);

        let raw_messages =
            if messages.is_array() && !messages.as_array().unwrap_or(&vec![]).is_empty() {
                info!(
                    "Using provided messages: {} messages ({} bytes)",
                    messages.as_array().map(|a| a.len()).unwrap_or(0),
                    messages.to_string().len()
                );
                messages
            } else {
                &default_messages
            };

        let context_limit = if model.contains("glm-4") || model.contains("GLM-4") {
            202750
        } else if model.contains("gemini") {
            1000000
        } else if model.contains("gpt-oss") || model.contains("gpt-4") {
            128000
        } else if model.contains("gpt-3.5") {
            16385
        } else if model.starts_with("http://localhost:808") || model == "local" {
            768
        } else {
            32768
        };

        let messages = OpenAIClient::ensure_token_limit(raw_messages, context_limit);

        let estimated_tokens = OpenAIClient::estimate_messages_tokens(&messages);
        if let Err(e) = self.rate_limiter.acquire(estimated_tokens).await {
            log::error!("Rate limit exceeded: {}", e);
            return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>);
        }

        let full_url = format!("{}{}", self.base_url, self.endpoint_path);
        let auth_header = format!("Bearer {}", key);

        trace!("LLM Request Details:");
        trace!(" URL: {}", full_url);
        trace!(" Authorization: Bearer <{} chars>", key.len());
        trace!(" Model: {}", model);
        if let Some(msg_array) = messages.as_array() {
            trace!(" Messages: {} messages", msg_array.len());
        }
        if let Some(tools) = tools {
            trace!(" Tools: {} tools provided", tools.len());
        }

        let token_key = if model.contains("gpt-5") {
            "max_completion_tokens"
        } else {
            "max_tokens"
        };
        // gpt-oss weights lose SSE frames to the streaming decoder on the
        // hosts below. Cerebras now comes from the catalog identity (#1467);
        // NVIDIA has no dedicated variant, so its host is still matched.
        let host_forces_no_stream = model.contains("gpt-oss")
            && (self.provider_type() == LLMProviderType::Cerebras
                || self.base_url.contains("nvidia"));
        let use_stream = !host_forces_no_stream
            // #1202 — the tokenrouter free GLM tier intermittently fails SSE
            // decoding ('Stream read error: error decoding response body');
            // its non-streaming path is reliable, so disable streaming for it.
            && !(model.contains("glm") && self.base_url.contains("tokenrouter"));
        if !use_stream {
            info!("Setting stream=false for model: {}", model);
        }
        let mut request_body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": use_stream,
            // Serving cap for this host, from the provider catalog rather than
            // an inline hostname check (#1467).
            token_key: self.max_output_tokens,
            "temperature": 1.0,
            "top_p": 1.0
        });

        // Only add chat_template_kwargs for native z.ai/GLM API, not for opencode.ai proxy
        if (model.contains("kimi") || model.contains("glm")) && self.base_url.contains("z.ai") {
            let kwargs = if model.contains("glm") {
                serde_json::json!({"enable_thinking": true, "clear_thinking": false})
            } else {
                serde_json::json!({"thinking": true})
            };
            request_body["chat_template_kwargs"] = kwargs;
            info!("Model factory: enabled thinking mode for {} (chat_template_kwargs)", model);
        }

        if let Some(tools_value) = tools {
            if !tools_value.is_empty() {
                request_body["tools"] = serde_json::json!(tools_value);
                request_body["tool_choice"] = serde_json::json!("auto");
                info!("Added {} tools to LLM request (tool_choice=auto)", tools_value.len());

                if let Some(first_tool) = tools_value.first().and_then(|t| t.get("function")) {
                    info!("First tool: name={:?}", first_tool.get("name"));
                }

                if model.contains("deepseek") && !self.base_url.contains("opencode.ai") {
                    let legacy_functions: Vec<Value> = tools_value
                        .iter()
                        .filter_map(|t| t.get("function").cloned())
                        .collect();
                    if !legacy_functions.is_empty() {
                        request_body["functions"] = serde_json::json!(legacy_functions);
                        trace!("Added {} legacy functions for DeepSeek compatibility", legacy_functions.len());
                    }
                }
            }
        }

        let (chunk_tx, chunk_rx) = tokio::sync::mpsc::channel::<Result<Vec<u8>, String>>(100000);

        // #1393 — serialize calls process-wide and back off properly on 429.
        let _gate = retry::call_gate().acquire().await;
        let max_retries = 6;
        let mut stream_started = false;
        info!("LLM request URL: {}, body size: {} bytes, stream={}", full_url, request_body.to_string().len(), use_stream);
        'retry_loop: for attempt in 0..=max_retries {
            let response = match self.client
                .post(&full_url)
                .header("Authorization", &auth_header)
                .header("Content-Type", "application/json")
                .json(&request_body)
                .send()
                .await
            {
                Ok(r) => r,
                    Err(e) => {
                        let err_msg = format!("HTTP error: {:#}", e);
                        let _ = chunk_tx.send(Err(err_msg.clone())).await;
                        if attempt < max_retries {
                            let wait = retry::backoff_ms(attempt);
                            warn!("LLM generate_stream attempt {} failed (connection): {} — retrying in {:?}", attempt + 1, err_msg, wait);
                            tokio::time::sleep(wait).await;
                            continue;
                        }
                        log::error!("LLM generate_stream failed after {} attempts: {}", attempt + 1, err_msg);
                        return Err("LLM stream request failed after retries".into());
                    }
            };

            let status = response.status();
            info!("LLM HTTP response status: {} for model {} (attempt {})", status, model, attempt + 1);
            if !status.is_success() {
                let retry_after_hdr = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok().map(str::to_string));
                let error_text = response.text().await.unwrap_or_default();
                warn!("LLM HTTP error body: {}", error_text);
                let _ = chunk_tx.send(Err(format!("Status {}: {}", status, error_text))).await;
                if attempt < max_retries && retry::is_retryable_status(status.as_u16()) {
                    let wait = retry::wait_for(status.as_u16(), &error_text, retry_after_hdr.as_deref(), attempt);
                    warn!("LLM generate_stream attempt {} failed (status {}): {} — retrying in {:?}", attempt + 1, status, error_text, wait);
                    tokio::time::sleep(wait).await;
                    continue;
                }
                log::error!("LLM generate_stream failed after {} attempts (status {}): {}", attempt + 1, status, error_text);
                return Err(format!("LLM request failed with status: {}: {}", status, error_text).into());
            }

            stream_started = true;

            if !use_stream {
                let body = response.text().await.map_err(|e| format!("Failed: {}", e))?;
                if let Ok(val) = serde_json::from_str::<Value>(&body) {
                    let choice = &val["choices"][0];
                    if let Some(reasoning) = choice["message"]["reasoning_content"].as_str().or(choice["message"]["reasoning"].as_str()) {
                        let _ = tx.send(serde_json::json!({"__reasoning__": reasoning}).to_string()).await;
                    }
                    if let Some(content) = choice["message"]["content"].as_str() {
                        if !content.is_empty() {
                            let _ = tx.send(content.to_string()).await;
                        }
                    }
                    if let Some(tcs) = choice["message"]["tool_calls"].as_array() {
                        for tc in tcs {
                            if let Some(func) = tc.get("function") {
                                let args = func["arguments"].as_str().unwrap_or("{}");
                                let msg = serde_json::json!({"__tool_call__": true, "name": func["name"], "arguments": args});
                                let _ = tx.send(msg.to_string()).await;
                            }
                        }
                    }
                }
                return Ok(());
            }

            let mut stream = response.bytes_stream();
            use futures_util::StreamExt;
            // Per-chunk read timeout: a provider that returns 200 and then
            // stalls the body used to hang this loop forever while holding
            // the process-wide call_gate, blocking every other LLM call.
            let read_timeout = std::time::Duration::from_secs(120);
            loop {
                let next = match tokio::time::timeout(read_timeout, stream.next()).await {
                    Ok(item) => item,
                    Err(_) => {
                        let err_msg = format!("Stream read stalled: no bytes for {read_timeout:?}");
                        log::error!("LLM generate_stream {err_msg}");
                        return Err(err_msg.into());
                    }
                };
                match next {
                    Some(Ok(bytes)) => {
                        if chunk_tx.send(Ok(bytes.to_vec())).await.is_err() {
                            break 'retry_loop;
                        }
                    }
                    Some(Err(e)) => {
                        let err_msg = format!("Stream read error: {}", e);
                        let _ = chunk_tx.send(Err(err_msg.clone())).await;
                        log::error!("LLM generate_stream stream error: {}", err_msg);
                        return Err(err_msg.into());
                    }
                    None => break,
                }
            }
            break;
        }

        if !stream_started {
            return Err("LLM stream request failed to start".into());
        }
        drop(chunk_tx);

        crate::openai_stream_sse::consume_sse_stream(chunk_rx, tx, model).await
    }

    async fn generate(
        &self,
        prompt: &str,
        messages: &Value,
        model: &str,
        key: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        crate::openai_stream_generate::generate(self, prompt, messages, model, key).await
    }

    async fn cancel_job(
        &self,
        _session_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}
