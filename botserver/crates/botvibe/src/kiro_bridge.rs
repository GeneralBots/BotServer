//! Bridge between the vibe agent loop and the single Kiro implementation.
//!
//! The Kiro (Amazon Q Developer / CodeWhisperer) protocol used to be
//! implemented twice — once in `botllm::kiro` and once here. Only the
//! `botllm` copy survives (#1498); this module keeps the agent-loop call
//! signature and translates the provider's stream events back into the
//! canonical envelope the agent loop parses.

use botllm::kiro::KiroClient;
use serde_json::{json, Value};
use tokio::sync::mpsc;

use crate::agent_loop::LlmUsage;

/// `true` when a configured LLM url points at Kiro rather than at an
/// OpenAI-compatible endpoint.
pub fn is_kiro(api_url: &str) -> bool {
    KiroClient::is_kiro_endpoint(api_url)
}

/// Sends one system/prompt/tools turn to Kiro and returns the agent loop's
/// canonical shape: plain content, or `{"tool_calls": [...]}` when the model
/// asked for a tool.
pub async fn call_kiro(
    api_url: &str,
    api_key: &str,
    model: &str,
    system: &str,
    prompt: &str,
    tools: &[Value],
    _max_retries: usize,
) -> Result<(String, Option<LlmUsage>), String> {
    let client = KiroClient::new(api_url.to_string());
    let config = json!([
        {"role": "system", "content": system},
        {"role": "user", "content": prompt},
    ]);
    let (tx, mut rx) = mpsc::channel::<String>(1024);
    let stream = {
        let client = client.clone();
        let config = config.clone();
        let prompt = prompt.to_string();
        let model = model.to_string();
        let api_key = api_key.to_string();
        let tools = tools.to_vec();
        tokio::spawn(async move {
            use botllm::LLMProvider;
            client
                .generate_stream(&prompt, &config, tx, &model, &api_key, Some(&tools))
                .await
        })
    };

    let mut content = String::new();
    let mut tool_calls: Vec<Value> = Vec::new();
    let mut stream_error: Option<String> = None;
    while let Some(chunk) = rx.recv().await {
        if let Some(err) = chunk.strip_prefix("__kiro_error__") {
            stream_error = Some(err.to_string());
            continue;
        }
        if chunk.starts_with("{\"__reasoning__") {
            continue;
        }
        if let Ok(event) = serde_json::from_str::<Value>(&chunk) {
            if event.get("__tool_call__").is_some() {
                let name = event.get("name").and_then(|n| n.as_str()).unwrap_or_default();
                let arguments = event
                    .get("arguments")
                    .and_then(|a| a.as_str())
                    .map(|a| serde_json::from_str::<Value>(a).unwrap_or_else(|_| json!({})))
                    .unwrap_or_else(|| json!({}));
                tool_calls.push(json!({"tool_name": name, "arguments": arguments}));
                continue;
            }
        }
        content.push_str(&chunk);
    }

    if let Err(e) = stream.await {
        stream_error.get_or_insert_with(|| e.to_string());
    }
    if let Some(err) = stream_error {
        if content.is_empty() && tool_calls.is_empty() {
            return Err(format!("Kiro stream error: {err}"));
        }
    }
    if content.is_empty() && tool_calls.is_empty() {
        return Err("Kiro returned empty response".to_string());
    }

    if !tool_calls.is_empty() {
        let canonical = json!({ "tool_calls": tool_calls }).to_string();
        return Ok((canonical, None));
    }
    Ok((content, None))
}
