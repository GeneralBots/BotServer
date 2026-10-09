use super::*;

impl VertexClient {
    pub fn new(base_url: String, endpoint_path: Option<String>) -> Self {
        let endpoint = endpoint_path.unwrap_or_else(|| {
            if base_url.contains("generativelanguage.googleapis.com") {
                // Default to OpenAI compatibility if possible, but generate_stream will auto-detect
                "/v1beta/openai/chat/completions".to_string()
            } else if base_url.contains("aiplatform.googleapis.com") && !base_url.contains("openapi") {
                // If it's a naked aiplatform URL, it might be missing the project/location path
                // We'll leave it empty and let the caller provide a full URL if needed
                "".to_string()
            } else {
                "".to_string()
            }
        });

        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(180))
                .connect_timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| {
                    log::warn!("vertex: failed to build reqwest client, using default");
                    reqwest::Client::new()
                }),
            base_url,
            endpoint_path: endpoint,
            token_manager: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    async fn get_auth_header(&self, key: &str) -> (&'static str, String) {
        // If the key is a path or JSON (starts with {), it's a Service Account
        if key.starts_with('{') || key.starts_with('~') || key.starts_with('/') || key.ends_with(".json") {
            let mut manager_opt = self.token_manager.write().await;
            if manager_opt.is_none() {
                *manager_opt = Some(Arc::new(VertexTokenManager::new(key)));
            }
            if let Some(manager) = manager_opt.as_ref() {
                match manager.get_access_token().await {
                    Ok(token) => return ("Authorization", format!("Bearer {}", token)),
                    Err(e) => log::error!("Failed to get Vertex OAuth token: {}", e),
                }
            }
        }
        
        // Default to Google API Key (Google AI Studio / Gemini Developer APIs)
        ("x-goog-api-key", key.to_string())
    }

    pub(super) fn convert_messages_to_gemini(&self, messages: &Value) -> Value {
        let mut contents = Vec::new();
        if let Some(msg_array) = messages.as_array() {
            for msg in msg_array {
                let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");
                let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
                
                let gemini_role = match role {
                    "user" => "user",
                    "assistant" => "model",
                    "system" | "episodic" | "compact" => "user",
                    "tool" => "user",
                    _ => "user",
                };

                contents.push(serde_json::json!({
                    "role": gemini_role,
                    "parts": [{"text": content}]
                }));
            }
        }
        serde_json::json!(contents)
    }
}
