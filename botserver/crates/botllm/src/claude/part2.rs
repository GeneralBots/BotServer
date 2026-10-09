use super::*;

impl ClaudeClient {
    async fn stream_with_reqwest(
        &self,
        request_body: String,
        api_key: &str,
        tx: mpsc::Sender<String>,
        model_name: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = if self.base_url.contains("/v1/messages") || self.base_url.contains("/messages") {
            self.base_url.clone()
        } else {
            format!("{}/v1/messages", self.base_url.trim_end_matches('/'))
        };
        let start_time = Instant::now();

        trace!(
            "CLAUDE Streaming request to {} body_len={}",
            url,
            request_body.len()
        );

        // Build client matching Node.js httpsAgent configuration
        // IMPORTANT: NO timeout() here - it causes premature stream closure!
        // Node.js doesn't set a timeout on the response body, only on connect/headers
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(TIMEOUT_MS))
            .pool_idle_timeout(Duration::from_secs(30)) // keepAliveMsecs: 30000
            .pool_max_idle_per_host(10) // maxFreeSockets: 10
            .tcp_nodelay(true)
            .tcp_keepalive(Duration::from_secs(60))
            .http1_only() // Force HTTP/1.1 like Node.js
            .build()?;

        // Send request with headers matching Node.js exactly
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Accept", "text/event-stream")
            .header("Connection", "keep-alive")
            .body(request_body)
            .send()
            .await?;

        let status = response.status();
        trace!("CLAUDE Response status: {} in {:?}", status, start_time.elapsed());

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::error!("Claude API error response: {}", error_text);
            return Err(format!("HTTP {}: {}", status, error_text).into());
        }

        // Stream response body - this is like Node.js res.on('data')
        let mut stream = response.bytes_stream();
        let mut sse_buffer = String::new();
        let mut text_chunks_sent = 0usize;
        let mut total_bytes = 0usize;
        let mut chunk_count = 0usize;
        let mut last_activity = Instant::now();
        let mut done_received = false;

        trace!("CLAUDE Starting stream read");

        loop {
            // Check overall timeout
            if start_time.elapsed() > Duration::from_millis(STREAMING_TIMEOUT_MS) {
                error!(
                    "CLAUDE Overall streaming timeout {}ms exceeded",
                    STREAMING_TIMEOUT_MS
                );
                break;
            }

            // Check activity timeout
            if last_activity.elapsed() > Duration::from_millis(ACTIVITY_TIMEOUT_MS) {
                if done_received || text_chunks_sent > 0 {
                    trace!(
                        "CLAUDE Activity timeout but {} chunks sent, treating as complete",
                        text_chunks_sent
                    );
                    break;
                }
                error!(
                    "CLAUDE Activity timeout - no data for {}ms",
                    ACTIVITY_TIMEOUT_MS
                );
                return Err(format!(
                    "Stream activity timeout - no data for {}ms",
                    ACTIVITY_TIMEOUT_MS
                )
                .into());
            }

            // Read next chunk with timeout - like Node.js res.on('data')
            let chunk_result = tokio::time::timeout(
                Duration::from_millis(ACTIVITY_TIMEOUT_MS),
                stream.next(),
            )
            .await;

            let chunk = match chunk_result {
                Ok(Some(Ok(bytes))) => {
                    last_activity = Instant::now();
                    bytes
                }
                Ok(Some(Err(e))) => {
                    log::error!("CLAUDE Stream read error: {}", e);
                    if text_chunks_sent > 0 {
                        warn!(
                            "CLAUDE Had {} chunks before error, treating as partial success",
                            text_chunks_sent
                        );
                        break;
                    }
                    return Err(format!("Stream read error: {}", e).into());
                }
                Ok(None) => {
                    trace!(
                        "CLAUDE Stream ended after {} chunks, {} bytes, {} text chunks",
                        chunk_count, total_bytes, text_chunks_sent
                    );
                    break;
                }
                Err(_) => {
                    if text_chunks_sent > 0 || done_received {
                        trace!(
                            "CLAUDE Timeout but {} chunks sent, treating as complete",
                            text_chunks_sent
                        );
                        break;
                    }
                    log::error!("CLAUDE Timeout waiting for stream data");
                    return Err("Timeout waiting for stream data".into());
                }
            };

            chunk_count += 1;
            total_bytes += chunk.len();
            let chunk_str = String::from_utf8_lossy(&chunk);



            sse_buffer.push_str(&chunk_str);

            // Process complete SSE events in buffer
            while let Some(event_end) = sse_buffer.find("\n\n") {
                let event_block = sse_buffer[..event_end].to_string();
                sse_buffer = sse_buffer[event_end + 2..].to_string();

                // Parse SSE event lines
                for line in event_block.lines() {
                    let line = line.trim();

                    if line.is_empty() || line.starts_with("event:") {
                        continue;
                    }

                    if !line.starts_with("data: ") {
                        continue;
                    }

                    let data = &line[6..];

                    if data == "[DONE]" {
                        trace!(
                            "CLAUDE Received DONE - {} chunks, {} bytes in {:?}",
                            text_chunks_sent,
                            total_bytes,
                            start_time.elapsed()
                        );
                        done_received = true;
                        continue;
                    }

                    // Process SSE data and send text chunks
                    let (content, reasoning) = self.process_sse_data(data, model_name);
                    
                    if let Some(text) = content {
                        if tx.send(text).await.is_err() {
                            warn!("CLAUDE Receiver dropped, stopping stream");
                            return Ok(());
                        }
                        text_chunks_sent += 1;
                    }
                    
                    if let Some(think) = reasoning {
                        let think_msg = serde_json::json!({
                            "type": "thinking",
                            "content": think
                        }).to_string();
                        if tx.send(think_msg).await.is_err() {
                            warn!("CLAUDE Receiver dropped, stopping stream");
                            return Ok(());
                        }
                    }
                }
            }
        }

        // Process any remaining data in buffer
        if !sse_buffer.is_empty() {
            for line in sse_buffer.lines() {
                let line = line.trim();
                if let Some(data) = line.strip_prefix("data: ") {
                    if data != "[DONE]" {
                        let (content, reasoning) = self.process_sse_data(data, model_name);
                        if let Some(text) = content {
                            let _ = tx.send(text).await;
                            text_chunks_sent += 1;
                        }
                        if let Some(think) = reasoning {
                            let think_msg = serde_json::json!({
                                "type": "thinking",
                                "content": think
                            }).to_string();
                            let _ = tx.send(think_msg).await;
                        }
                    }
                }
            }
        }

        trace!(
            "CLAUDE Stream complete: {} chunks, {} bytes, {} text in {:?}",
            chunk_count, total_bytes, text_chunks_sent, start_time.elapsed()
        );

        if text_chunks_sent == 0 && !done_received {
            warn!("CLAUDE No text chunks sent and no DONE received");
        }

        Ok(())
    }

    /// Single streaming attempt with full error handling
    async fn stream_single_attempt(
        &self,
        prompt: &str,
        messages: &Value,
        tx: mpsc::Sender<String>,
        model: &str,
        key: &str,
        _tools: Option<&Vec<Value>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let model_name = if model.is_empty() {
            &self.deployment_name
        } else {
            model
        };
        let (system, claude_messages) = self.build_messages_from_value(prompt, messages);

        if claude_messages.is_empty() {
            return Err("No messages to send".into());
        }

        let thinking = if model_name.contains("3-7") || model_name.contains("4-7") {
            Some(ClaudeThinking {
                thinking_type: "enabled".to_string(),
                budget_tokens: 2048,
            })
        } else {
            None
        };

        let request = ClaudeRequest {
            model: model_name.to_string(),
            max_tokens: if thinking.is_some() { 16000 } else { 4096 },
            messages: claude_messages,
            system,
            stream: Some(true),
            thinking,
        };

        let request_body = serde_json::to_string(&request)?;
        trace!(
            "CLAUDE Streaming request: model={}, messages={}, body_len={}",
            model_name,
            request.messages.len(),
            request_body.len()
        );

        self.stream_with_reqwest(request_body, key, tx, model_name)
            .await
    }
}
