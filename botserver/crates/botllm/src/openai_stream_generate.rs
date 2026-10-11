//! Non-streaming `generate` path for `OpenAIClient`.
//! Split from openai_stream.rs for file-size limits.

use log::{trace, warn};
use serde_json::Value;

use crate::llm_models::get_handler;
use crate::openai_client::OpenAIClient;
use crate::retry;

/// Sends one non-streaming chat completion request and returns the content.
pub(crate) async fn generate(
    client: &OpenAIClient,
    prompt: &str,
    messages: &Value,
    model: &str,
    key: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let default_messages = serde_json::json!([{"role": "user", "content": prompt}]);

        let raw_messages =
            if messages.is_array() && !messages.as_array().unwrap_or(&vec![]).is_empty() {
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

        let full_url = format!("{}{}", client.base_url, client.endpoint_path);
        let auth_header = format!("Bearer {}", key);

        trace!("LLM Request Details:");
        trace!(" URL: {}", full_url);
        trace!(" Authorization: Bearer <{} chars>", key.len());
        trace!(" Model: {}", model);
        if let Some(msg_array) = messages.as_array() {
            trace!(" Messages: {} messages", msg_array.len());
        }

        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": false
        });

        // #1393 — serialize calls process-wide (chat + vibe + scaffold share
        // one provider key; free tiers enforce per-client concurrency ~1).
        let _gate = retry::call_gate().acquire().await;
        let max_retries = 6;
        let mut result = None;
        for attempt in 0..=max_retries {
            let outcome = async {
                let resp = client.client
                    .post(&full_url)
                    .header("Authorization", &auth_header)
                    .header("Content-Type", "application/json")
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| format!("HTTP error: {}", e))?;
                let status = resp.status();
                let retry_after = resp
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok().map(str::to_string));
                let text = resp.text().await.unwrap_or_default();
                Ok::<(u16, String, Option<String>), String>((status.as_u16(), text, retry_after))
            }.await;

            match outcome {
                Ok((200, text, _)) => {
                    result = Some(text);
                    break;
                }
                Ok((status, error_text, retry_after_hdr)) => {
                    if attempt < max_retries && retry::is_retryable_status(status) {
                        let wait = retry::wait_for(status, &error_text, retry_after_hdr.as_deref(), attempt);
                        warn!("LLM generate attempt {} failed (status {}): {} — retrying in {:?}", attempt + 1, status, error_text, wait);
                        tokio::time::sleep(wait).await;
                    } else {
                        log::error!("LLM generate error after {} attempts (status {}): {}", attempt + 1, status, error_text);
                        return Err(format!("LLM request failed with status: {}: {}", status, error_text).into());
                    }
                }
                Err(e) => {
                    let err_detail = format!("{:#}", e);
                    if attempt < max_retries {
                        let wait = retry::backoff_ms(attempt);
                        warn!("LLM generate attempt {} failed (connection): {} — retrying in {:?}", attempt + 1, err_detail, wait);
                        tokio::time::sleep(wait).await;
                    } else {
                        log::error!("LLM generate connection error after {} attempts: {}", attempt + 1, err_detail);
                        return Err(format!("LLM error: {}", err_detail).into());
                    }
                }
            }
        }

        let response_text = result.ok_or_else(|| {
            Box::<dyn std::error::Error + Send + Sync>::from(format!(
                "LLM request failed after {} retries", max_retries
            ))
        })?;

        let result_value: Value = serde_json::from_str(&response_text)
            .map_err(|e| format!("JSON parse error: {}", e))?;
        let msg = &result_value["choices"][0]["message"];
        let raw_content = msg["content"]
            .as_str()
            .filter(|s| !s.is_empty())
            .or_else(|| msg["reasoning_content"].as_str())
            .or_else(|| msg["reasoning"].as_str())
            .unwrap_or("");

        let handler = get_handler(model);
        let content = handler.process_content(raw_content);

        Ok(content)
}
