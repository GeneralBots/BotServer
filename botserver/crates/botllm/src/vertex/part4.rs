use super::*;

#[async_trait]
impl LLMProvider for VertexClient {
    async fn generate(
        &self,
        prompt: &str,
        messages: &Value,
        model: &str,
        key: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let default_messages = serde_json::json!([{"role": "user", "content": prompt}]);
        let raw_messages = if messages.is_array() && !messages.as_array().unwrap_or(&vec![]).is_empty() {
            messages
        } else {
            &default_messages
        };

        let (header_name, auth_value) = self.get_auth_header(key).await;
        
        // Auto-detect if we should use native Gemini API
        let is_native = self.endpoint_path.contains(":generateContent") || 
                        self.base_url.contains("generativelanguage") && !self.endpoint_path.contains("openai");

        let full_url = if is_native && !self.endpoint_path.contains(":generateContent") {
            // Build native URL for generativelanguage
            format!("{}/v1beta/models/{}:generateContent", self.base_url.trim_end_matches('/'), model)
        } else {
            format!("{}{}", self.base_url, self.endpoint_path)
        }.trim_end_matches('/').to_string();

        let request_body = if is_native {
            serde_json::json!({
                "contents": self.convert_messages_to_gemini(raw_messages),
                "generationConfig": {
                    "temperature": 0.7,
                }
            })
        } else {
            serde_json::json!({
                "model": model,
                "messages": raw_messages,
                "stream": false
            })
        };

        info!("Sending request to Vertex/Gemini endpoint: {}", full_url);

        let response = self.client
            .post(full_url)
            .header(header_name, &auth_value)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::error!("Vertex/Gemini generate error: {}", error_text);
            return Err(format!("Google API error ({}): {}", status, error_text).into());
        }

        let json: Value = response.json().await?;
        
        // Try parsing OpenAI format
        if let Some(choices) = json.get("choices") {
            if let Some(first_choice) = choices.get(0) {
                if let Some(message) = first_choice.get("message") {
                    if let Some(content) = message.get("content") {
                        if let Some(content_str) = content.as_str() {
                            return Ok(content_str.to_string());
                        }
                    }
                }
            }
        }
        
        // Try parsing Native Gemini format
        if let Some(candidates) = json.get("candidates") {
            if let Some(first_candidate) = candidates.get(0) {
                if let Some(content) = first_candidate.get("content") {
                    if let Some(parts) = content.get("parts") {
                        if let Some(first_part) = parts.get(0) {
                            if let Some(text) = first_part.get("text") {
                                if let Some(text_str) = text.as_str() {
                                    return Ok(text_str.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        Err("Failed to parse response from Vertex/Gemini (unknown format)".into())
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        messages: &Value,
        tx: mpsc::Sender<String>,
        model: &str,
        key: &str,
        tools: Option<&Vec<Value>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let default_messages = serde_json::json!([{"role": "user", "content": prompt}]);
        let raw_messages = if messages.is_array() && !messages.as_array().unwrap_or(&vec![]).is_empty() {
            messages
        } else {
            &default_messages
        };

        let (header_name, auth_value) = self.get_auth_header(key).await;
        
        // Auto-detect if we should use native Gemini API
        let is_native = self.endpoint_path.contains("GenerateContent") || 
                        (self.base_url.contains("googleapis.com") && 
                         !self.endpoint_path.contains("openai") && 
                         !self.endpoint_path.contains("completions"));

        let full_url = if is_native && !self.endpoint_path.contains("GenerateContent") {
             // Build native URL for Gemini if it looks like we need it
             if self.base_url.contains("aiplatform") {
                 // Global Vertex endpoint format
                 format!("{}/v1/publishers/google/models/{}:streamGenerateContent", self.base_url.trim_end_matches('/'), model)
             } else {
                 // Google AI Studio format
                 format!("{}/v1beta/models/{}:streamGenerateContent", self.base_url.trim_end_matches('/'), model)
             }
        } else {
            format!("{}{}", self.base_url, self.endpoint_path)
        }.trim_end_matches('/').to_string();

        let mut request_body = if is_native {
            let mut body = serde_json::json!({
                "contents": self.convert_messages_to_gemini(raw_messages),
                "generationConfig": {
                    "temperature": 0.7,
                }
            });
            
            // Handle thinking models if requested
            if model.contains("preview") || model.contains("3.1-flash") {
                body["generationConfig"]["thinkingConfig"] = serde_json::json!({
                    "thinkingLevel": "LOW"
                });
            }
            body
        } else {
            serde_json::json!({
                "model": model,
                "messages": raw_messages,
                "stream": true
            })
        };

        if let Some(tools_value) = tools {
            if !tools_value.is_empty() {
                request_body["tools"] = serde_json::json!(tools_value);
                info!("Added {} tools to Vertex request", tools_value.len());
            }
        }

        info!("Sending streaming request to Vertex/Gemini endpoint: {}", full_url);

        let response = self.client
            .post(full_url)
            .header(header_name, &auth_value)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            log::error!("Vertex/Gemini generate_stream error: {}", error_text);
            return Err(format!("Google API error ({}): {}", status, error_text).into());
        }

        let mut stream = response.bytes_stream();
        let mut tool_call_buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    if let Ok(text) = std::str::from_utf8(&chunk) {
                        for line in text.split('\n') {
                            let line = line.trim();
                            if line.is_empty() { continue; }

                            if let Some(data) = line.strip_prefix("data: ") {
                                // --- OpenAI SSE Format ---
                                if data == "[DONE]" { continue; }
                                if let Ok(json) = serde_json::from_str::<Value>(data) {
                                    if let Some(choices) = json.get("choices") {
                                        if let Some(first_choice) = choices.get(0) {
                                            if let Some(delta) = first_choice.get("delta") {
                                                if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                                                    if !content.is_empty() {
                                                        let _ = tx.send(content.to_string()).await;
                                                    }
                                                }
                                                
                                                // Handle tool calls...
                                                if let Some(tool_calls) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                                                    if let Some(first_call) = tool_calls.first() {
                                                        if let Some(function) = first_call.get("function") {
                                                            if let Some(name) = function.get("name").and_then(|n| n.as_str()) {
                                                                tool_call_buffer = format!("{{\"name\": \"{}\", \"arguments\": \"", name);
                                                            }
                                                            if let Some(args) = function.get("arguments").and_then(|a| a.as_str()) {
                                                                tool_call_buffer.push_str(args);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                // --- Native Gemini JSON format ---
                                // It usually arrives as raw JSON objects, sometimes with leading commas or brackets in a stream array
                                let trimmed = line.trim_start_matches([',', '[', ']']).trim_end_matches(']');
                                if trimmed.is_empty() { continue; }
                                
                                if let Ok(json) = serde_json::from_str::<Value>(trimmed) {
                                    if let Some(candidates) = json.get("candidates").and_then(|c| c.as_array()) {
                                        if let Some(candidate) = candidates.first() {
                                            if let Some(content) = candidate.get("content") {
                                                if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                                                    for part in parts {
                                                        if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                                            if !text.is_empty() {
                                                                let _ = tx.send(text.to_string()).await;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("Vertex stream reading error: {}", e);
                    break;
                }
            }
        }
        
        if !tool_call_buffer.is_empty() {
            tool_call_buffer.push_str("\"}");
            let _ = tx.send(format!("`tool_call`: {}", tool_call_buffer)).await;
        }

        Ok(())
    }

    async fn cancel_job(&self, _session_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}
