use super::*;

#[async_trait]
impl LLMProvider for ClaudeClient {
    async fn generate(
        &self,
        prompt: &str,
        messages: &Value,
        model: &str,
        key: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let model_name = if model.is_empty() {
            &self.deployment_name
        } else {
            model
        };
        let (system, claude_messages) = self.build_messages_from_value(prompt, messages);

        if claude_messages.is_empty() {
            return Err("No messages to send".into());
        }

        let request = ClaudeRequest {
            model: model_name.to_string(),
            max_tokens: 4096,
            messages: claude_messages,
            system,
            stream: None,
            thinking: None,
        };

        let body = serde_json::to_string(&request)?;
        trace!("CLAUDE request: model={}, body_len={}", model_name, body.len());

        let start = Instant::now();
        let url = if self.base_url.contains("/v1/messages") || self.base_url.contains("/messages") {
            self.base_url.clone()
        } else {
            format!("{}/v1/messages", self.base_url.trim_end_matches('/'))
        };

        // Use reqwest for non-streaming
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(TIMEOUT_MS))
            .build()?;

        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .body(body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Claude API error ({}): {}", status, error_text).into());
        }

        trace!("CLAUDE response in {:?}, status={}", start.elapsed(), status);

        let result: ClaudeResponse = response.json().await?;
        let content = self.extract_text_from_response(&result);
        let handler = get_handler(model_name);

        Ok(handler.process_content(&content))
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        messages: &Value,
        tx: mpsc::Sender<String>,
        model: &str,
        key: &str,
        _tools: Option<&Vec<Value>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut last_error: Option<Box<dyn std::error::Error + Send + Sync>> = None;

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                let delay = Self::get_retry_delay(attempt - 1);
                trace!(
                    "CLAUDE Retry {}/{} in {:?}",
                    attempt, MAX_RETRIES, delay
                );
                tokio::time::sleep(delay).await;
            }

            match self
                .stream_single_attempt(prompt, messages, tx.clone(), model, key, _tools)
                .await
            {
                Ok(()) => {
                    if attempt > 0 {
                        trace!("CLAUDE Success after {} attempts", attempt + 1);
                    }
                    return Ok(());
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    log::error!("CLAUDE Attempt {} failed: {}", attempt + 1, err_msg);

                    if attempt < MAX_RETRIES && Self::is_retryable_error(&err_msg, None) {
                        last_error = Some(e);
                        continue;
                    }

                    return Err(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| "Max retries exceeded".into()))
    }

    async fn cancel_job(
        &self,
        _session_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}
