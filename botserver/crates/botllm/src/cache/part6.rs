use super::*;

impl LocalEmbeddingService {
    pub fn new(embedding_url: String, model: String, api_key: Option<String>) -> Self {
        Self {
            embedding_url,
            model,
            api_key,
        }
    }

    /// Generate a deterministic hash-based embedding for fallback
    pub(super) fn hash_embedding(&self, text: &str) -> Vec<f32> {
        const EMBEDDING_DIM: usize = 384; // Match common embedding dimensions
        let mut embedding = vec![0.0f32; EMBEDDING_DIM];

        let hash = Sha256::digest(text.as_bytes());

        // Use hash bytes to seed the embedding
        for (i, byte) in hash.iter().cycle().take(EMBEDDING_DIM * 4).enumerate() {
            let idx = i % EMBEDDING_DIM;
            let value = (*byte as f32 - 128.0) / 128.0;
            embedding[idx] += value * 0.1;
        }

        // Normalize
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut embedding {
                *val /= norm;
            }
        }

        embedding
    }
}
#[async_trait]
impl EmbeddingService for LocalEmbeddingService {
    async fn get_embedding(
        &self,
        text: &str,
    ) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::new();

        // Determine if URL already includes endpoint path
        let url = if self.embedding_url.contains("/pipeline/") ||
                   self.embedding_url.contains("/v1/") ||
                   self.embedding_url.contains("/ai/run/") ||
                   self.embedding_url.ends_with("/embeddings") {
            self.embedding_url.clone()
        } else {
            format!("{}/embedding", self.embedding_url)
        };

        // Determine request body format based on URL
        let request_body = if self.embedding_url.contains("huggingface.co") {
            serde_json::json!({
                "inputs": text,
            })
        } else if self.embedding_url.contains("/ai/run/") {
            // Cloudflare AI format
            serde_json::json!({
                "text": text,
            })
        } else {
            serde_json::json!({
                "input": text,
                "model": self.model,
            })
        };

        // Retry logic with exponential backoff
        const MAX_RETRIES: u32 = 3;
        const INITIAL_DELAY_MS: u64 = 500;

        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                let delay_ms = INITIAL_DELAY_MS * (1 << (attempt - 1)); // 500, 1000, 2000
                debug!("Embedding service retry attempt {}/{} after {}ms", attempt + 1, MAX_RETRIES, delay_ms);
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            }

            let mut request = client.post(&url);

            // Add authorization header if API key is provided
            if let Some(ref api_key) = self.api_key {
                request = request.header("Authorization", format!("Bearer {}", api_key));
            }

            match request
                .json(&request_body)
                .timeout(std::time::Duration::from_secs(30))
                .send()
                .await
            {
                Ok(response) => {
                    let status = response.status();
                    let response_text = match response.text().await {
                        Ok(t) => t,
                        Err(e) => {
                            debug!("Failed to read response body: {}", e);
                            continue;
                        }
                    };

                    if !status.is_success() {
                        debug!(
                            "Embedding service HTTP error {} (attempt {}/{}): {}",
                            status, attempt + 1, MAX_RETRIES, response_text
                        );
                        // Retry on 5xx errors
                        if status.as_u16() >= 500 {
                            continue;
                        }
                        // Non-retriable error
                        return Err(format!(
                            "Embedding service returned HTTP {}: {}",
                            status, response_text
                        ).into());
                    }

                    // Success - parse response
                    let result: Value = match serde_json::from_str(&response_text) {
                        Ok(r) => r,
                        Err(e) => {
                            debug!("Failed to parse embedding JSON: {} - Response: {}", e, response_text);
                            return Err(format!("Failed to parse embedding response JSON: {} - Response: {}", e, response_text).into());
                        }
                    };

                    if let Some(error) = result.get("error") {
                        debug!("Embedding service returned error: {}", error);
                        return Err(format!("Embedding service error: {}", error).into());
                    }

                    // Try multiple response formats
                    let embedding = if let Some(arr) = result.as_array() {
                        // HuggingFace format: direct array [0.1, 0.2, ...]
                        arr.iter()
                            .filter_map(|v| v.as_f64().map(|f| f as f32))
                            .collect()
                    } else if let Some(result_obj) = result.get("result") {
                        // Cloudflare AI format: {"result": {"data": [[...]]}}
                        if let Some(data) = result_obj.get("data") {
                            if let Some(data_arr) = data.as_array() {
                                if let Some(first) = data_arr.first() {
                                    if let Some(embedding_arr) = first.as_array() {
                                        embedding_arr
                                            .iter()
                                            .filter_map(|v| v.as_f64().map(|f| f as f32))
                                            .collect()
                                    } else {
                                        data_arr
                                            .iter()
                                            .filter_map(|v| v.as_f64().map(|f| f as f32))
                                            .collect()
                                    }
                                } else {
                                    return Err("Empty data array in Cloudflare response".into());
                                }
                            } else {
                                return Err(format!("Invalid Cloudflare response format - Expected result.data array, got: {}", response_text).into());
                            }
                        } else {
                            return Err(format!("Invalid Cloudflare response format - Expected result.data, got: {}", response_text).into());
                        }
                    } else if let Some(data) = result.get("data") {
                        // OpenAI/Standard format: {"data": [{"embedding": [...]}]}
                        data[0]["embedding"]
                            .as_array()
                            .ok_or_else(|| {
                                debug!("Invalid embedding response format. Expected data[0].embedding array. Got: {}", response_text);
                                format!("Invalid embedding response format - Expected data[0].embedding array, got: {}", response_text)
                            })?
                            .iter()
                            .filter_map(|v| v.as_f64().map(|f| f as f32))
                            .collect()
                    } else {
                        return Err(format!(
                            "Invalid embedding response format - Expected array or data[0].embedding, got: {}",
                            response_text
                        ).into());
                    };

                    return Ok(embedding);
                }
                Err(e) => {
                    // Network error - retry
                    debug!("Embedding service network error (attempt {}/{}): {}", attempt + 1, MAX_RETRIES, e);
                }
            }
        }

        // All retries exhausted - use hash-based fallback
        debug!("Embedding service failed after all retries, using hash-based fallback");
        Ok(self.hash_embedding(text))
    }

    async fn compute_similarity(&self, embedding1: &[f32], embedding2: &[f32]) -> f32 {
        if embedding1.len() != embedding2.len() {
            return 0.0;
        }

        let dot_product: f32 = embedding1
            .iter()
            .zip(embedding2.iter())
            .map(|(a, b)| a * b)
            .sum();

        let norm1: f32 = embedding1.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm2: f32 = embedding2.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm1 == 0.0 || norm2 == 0.0 {
            return 0.0;
        }

        dot_product / (norm1 * norm2)
    }
}
