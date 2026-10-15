use super::*;

/// Finds the matching `}` for the `{` at `start` (brace-balanced, string-aware).
pub(super) fn find_json_end(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escape = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        let c = b as char;
        if escape {
            escape = false;
            continue;
        }
        match c {
            '\\' if in_string => escape = true,
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}
#[async_trait]
impl LLMProvider for KiroClient {
    async fn generate(
        &self,
        prompt: &str,
        config: &Value,
        model: &str,
        key: &str,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let raw_messages = if config.is_array() && !config.as_array().unwrap_or(&vec![]).is_empty() {
            config.clone()
        } else {
            json!([{"role": "user", "content": prompt}])
        };
        let model_id = Self::kiro_model_id(model);
        let request = Self::build_request(&raw_messages, &model_id, &[]);
        let stream = self.request_stream(&request, key).await?;
        let (tx, mut rx) = mpsc::channel::<String>(100_000);
        let handler = get_handler(model);
        let self_ = std::sync::Arc::new(self_clone(self));
        let emit = tokio::spawn(async move {
            let _ = self_.emit_stream(stream, &tx, handler.as_ref()).await;
        });
        let mut full = String::new();
        while let Some(chunk) = rx.recv().await {
            if chunk.starts_with("{\"__tool_call__") || chunk.starts_with("__kiro_error__") || chunk.starts_with("{\"__reasoning__") {
                if let Some(err) = chunk.strip_prefix("__kiro_error__") {
                    return Err(format!("Kiro stream error: {err}").into());
                }
                continue;
            }
            full.push_str(&chunk);
        }
        let _ = emit.await;
        Ok(full)
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        config: &Value,
        tx: mpsc::Sender<String>,
        model: &str,
        key: &str,
        tools: Option<&Vec<Value>>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let raw_messages = if config.is_array() && !config.as_array().unwrap_or(&vec![]).is_empty() {
            config.clone()
        } else {
            json!([{"role": "user", "content": prompt}])
        };
        let model_id = Self::kiro_model_id(model);
        let request = Self::build_request(&raw_messages, &model_id, tools.map(Vec::as_slice).unwrap_or(&[]));
        if let Err(e) = self.rate_limiter.acquire(1).await {
            error!("Kiro rate limit: {e}");
            return Err(Box::new(e));
        }
        let stream = self.request_stream(&request, key).await?;
        let handler = get_handler(model);
        self.emit_stream(stream, &tx, handler.as_ref()).await
    }

    async fn cancel_job(
        &self,
        _session_id: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }
}
/// KiroClient is used behind Arc by the emit task in `generate`; provide a
/// cheap clone (all fields are cheap handles).
pub(super) fn self_clone(client: &KiroClient) -> KiroClient {
    KiroClient {
        http: client.http.clone(),
        endpoint: client.endpoint.clone(),
        rate_limiter: client.rate_limiter.clone(),
    }
}
impl Clone for KiroClient {
    fn clone(&self) -> Self {
        self_clone(self)
    }
}
