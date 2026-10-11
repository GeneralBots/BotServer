use super::*;

impl KiroClient {
    pub fn new(base_url: String) -> Self {
        let endpoint = if base_url.trim().is_empty()
            || base_url.contains("kiro.dev")
            || !base_url.starts_with("http")
        {
            DEFAULT_ENDPOINT.to_string()
        } else {
            base_url.trim_end_matches('/').to_string() + "/"
        };
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .connect_timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| {
                warn!("Kiro: failed to build reqwest client with timeouts, using default");
                reqwest::Client::new()
            });
        Self {
            http,
            endpoint,
            rate_limiter: Arc::new(crate::rate_limiter::ApiRateLimiter::unlimited()),
        }
    }

    /// Normalizes a requested model id to a Kiro model id. Kiro exposes
    /// Claude-family models; dash/dot/versioned names all resolve to the
    /// canonical catalog id.
    pub(super) fn kiro_model_id(model: &str) -> String {
        let m = model.to_lowercase();
        // Strip version suffixes like "-20250929".
        let base = m
            .split('-')
            .filter(|p| !p.chars().all(|c| c.is_ascii_digit()) || p.len() != 8)
            .collect::<Vec<_>>()
            .join("-");
        let base = base.replace(".5", "-5").replace(".4", "-4");
        if base.contains("opus") {
            "claude-opus-5".to_string()
        } else if base.contains("haiku") {
            "claude-haiku-4-5".to_string()
        } else if base.contains("sonnet-4-5") || base.contains("sonnet-4.5") {
            "claude-sonnet-4-5".to_string()
        } else if base.contains("sonnet") {
            "claude-sonnet-4-6".to_string()
        } else if base.contains("glm") {
            "GLM-5".to_string()
        } else if base.contains("deepseek") {
            "DeepSeek-V3.2".to_string()
        } else if base.contains("qwen") {
            "Qwen3-Coder-Next".to_string()
        } else if base.contains("minimax") {
            "MiniMax-M2.5".to_string()
        } else {
            "auto".to_string()
        }
    }

    pub(super) fn is_capacity_error(body: &str) -> bool {
        body.contains("INSUFFICIENT_MODEL_CAPACITY")
    }

    /// Extracts the error message text from an AWS JSON error body.
    pub(super) fn extract_error_text(body: &str) -> String {
        if let Ok(v) = serde_json::from_str::<Value>(body) {
            let msg = v
                .get("message")
                .or_else(|| v.get("Message"))
                .or_else(|| v.get("__type"))
                .and_then(|m| m.as_str())
                .unwrap_or("");
            if !msg.is_empty() {
                return msg.to_string();
            }
        }
        body.chars().take(400).collect()
    }

    /// Single streaming request → parsed events. Capacity errors are retried
    /// with exponential backoff; everything else fails fast.
    pub(crate) async fn request_stream(
        &self,
        request: &Value,
        key: &str,
    ) -> Result<impl futures_util::Stream<Item = reqwest::Result<bytes::Bytes>>, Box<dyn Error + Send + Sync>>
    {
        let capacity_retries = Arc::new(std::sync::atomic::AtomicU32::new(0));
        for attempt in 0..=MAX_CAPACITY_RETRIES {
            let ua = format!(
                "aws-sdk-rust/1.0.0 ua/2.1 os/other lang/rust api/codewhispererstreaming#1.28.3 m/E app/AmazonQ-For-CLI md/appVersion-1.28.3-{}",
                uuid::Uuid::new_v4().simple()
            );
            let response = self
                .http
                .post(&self.endpoint)
                .header("Content-Type", "application/x-amz-json-1.0")
                .header("Accept", "application/json")
                .header("Authorization", format!("Bearer {key}"))
                .header("tokentype", "API_KEY")
                .header("X-Amz-Target", TARGET_STREAMING)
                .header("x-amzn-codewhisperer-optout", "true")
                .header("amz-sdk-invocation-id", uuid::Uuid::new_v4().to_string())
                .header("amz-sdk-request", "attempt=1; max=1")
                .header("x-amzn-kiro-agent-mode", "vibe")
                .header("x-amz-user-agent", &ua)
                .header("user-agent", &ua)
                .json(request)
                .send()
                .await;

            match response {
                Ok(r) if r.status().is_success() => {
                    return Ok(r.bytes_stream());
                }
                Ok(r) => {
                    let status = r.status();
                    let body = r.text().await.unwrap_or_default();
                    if Self::is_capacity_error(&body) {
                        let n = capacity_retries.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if n < MAX_CAPACITY_RETRIES {
                            let delay = Duration::from_millis(800 * 2u64.pow(n));
                            warn!("Kiro: model capacity error, retry {}/{} in {:?}", n + 1, MAX_CAPACITY_RETRIES, delay);
                            tokio::time::sleep(delay).await;
                            continue;
                        }
                    }
                    let msg = Self::extract_error_text(&body);
                    error!("Kiro request failed: status {status}: {msg}");
                    if status.as_u16() == 401 || status.as_u16() == 403 {
                        return Err(format!("Kiro API key rejected ({status}) — check the ksk_ key in Vault: {msg}").into());
                    }
                    return Err(format!("Kiro API error ({status}): {msg}").into());
                }
                Err(e) => {
                    if attempt < MAX_CAPACITY_RETRIES {
                        warn!("Kiro connection error ({e}), retrying…");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                    return Err(format!("Kiro connection failed: {e}").into());
                }
            }
        }
        Err("Kiro request failed after retries".into())
    }

    /// Builds the Kiro request from OpenAI-style messages.
    pub(super) fn build_request(messages: &Value, model_id: &str) -> Value {
        let empty = vec![];
        let arr = messages.as_array().unwrap_or(&empty);
        // System content is folded into the first user message (Kiro has no
        // system role).
        let system_text: String = arr
            .iter()
            .filter(|m| m.get("role").and_then(|r| r.as_str()) == Some("system"))
            .filter_map(|m| m.get("content").and_then(|c| c.as_str()))
            .collect::<Vec<_>>()
            .join("\n\n");

        let mut history: Vec<Value> = Vec::new();
        let mut current_content = String::new();

        for msg in arr {
            let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
            let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
            match role {
                "system" => {}
                "user" => {
                    if !current_content.is_empty() || !history.is_empty() {
                        // Flush a pending assistant turn before a new user turn
                        // keeps the strict alternation Kiro expects.
                    }
                    current_content = if history.is_empty() && !system_text.is_empty() {
                        format!("{system_text}\n\n{content}")
                    } else {
                        content.to_string()
                    };
                    history.push(json!({
                        "userInputMessage": {
                            "content": current_content,
                            "modelId": model_id,
                            "origin": ORIGIN,
                        }
                    }));
                }
                "assistant" => {
                    history.push(json!({
                        "assistantResponseMessage": { "content": content }
                    }));
                }
                "tool" => {
                    // Tool results are surfaced as plain context (chat tools
                    // are injected via __api_call__, not native tool_use).
                    let last = history.last_mut();
                    if let Some(last) = last {
                        if let Some(uim) = last.get_mut("userInputMessage") {
                            if let Some(c) = uim.get_mut("content").and_then(|c| c.as_str()) {
                                let merged = format!("{}\n\n[tool result] {content}", c);
                                uim["content"] = json!(merged);
                                continue;
                            }
                        }
                    }
                    history.push(json!({
                        "userInputMessage": {
                            "content": format!("[tool result] {content}"),
                            "modelId": model_id,
                            "origin": ORIGIN,
                        }
                    }));
                }
                _ => {}
            }
        }

        // The last entry is the CURRENT user turn — pop it out of history so
        // it becomes currentMessage (Kiro requires exactly one).
        let current = match history.pop() {
            Some(c) => c,
            None => json!({
                "userInputMessage": {
                    "content": "Hello",
                    "modelId": model_id,
                    "origin": ORIGIN,
                }
            }),
        };
        let current_user = current;

        let mut conversation = json!({
            "chatTriggerType": "MANUAL",
            "agentTaskType": "vibe",
            "currentMessage": { "userInputMessage": current_user["userInputMessage"].clone() },
        });
        if !history.is_empty() {
            conversation["history"] = json!(history);
        }
        json!({
            "conversationState": conversation,
            "agentMode": "vibe",
        })
    }
}
