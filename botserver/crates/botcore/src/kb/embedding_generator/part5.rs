use super::*;

impl KbEmbeddingGenerator {
    pub(crate) async fn generate_local_embeddings(&self, texts: &[String]) -> Result<Vec<Embedding>> {
        // Apply token-aware truncation to each text before creating request
        let truncated_texts: Vec<String> = texts.iter()
            .map(|text| crate::shared::utils::truncate_text_for_model(text, &self.config_read().embedding_model.clone(), 600))
            .collect();

        // Detect API format based on URL pattern
        // Cloudflare AI: https://api.cloudflare.com/client/v4/accounts/{account_id}/ai/run/@cf/baai/bge-m3
        // Scaleway (OpenAI-compatible): https://router.huggingface.co/scaleway/v1/embeddings
        // HuggingFace Inference (old): https://router.huggingface.co/hf-inference/models/.../pipeline/feature-extraction
        let is_cloudflare = self.config_read().embedding_url.clone().contains("api.cloudflare.com/client/v4/accounts");
        let is_scaleway = self.config_read().embedding_url.clone().contains("/scaleway/v1/embeddings");
        let is_hf_inference = self.config_read().embedding_url.clone().contains("/hf-inference/") ||
                             self.config_read().embedding_url.clone().contains("/pipeline/feature-extraction");

        let response = if is_cloudflare {
            // Cloudflare AI Workers API format: {"text": ["text1", "text2", ...]}
            let cf_request = CloudflareEmbeddingRequest {
                text: truncated_texts,
            };

            let request_size = serde_json::to_string(&cf_request)
                .map(|s| s.len())
                .unwrap_or(0);
            trace!("Sending Cloudflare AI request to {} (size: {} bytes)",
                  self.config_read().embedding_url.clone(), request_size);

            let mut request_builder = self.client
                .post(&self.config_read().embedding_url.clone())
                .json(&cf_request);

            // Add Authorization header if API key is provided
            if let Some(ref api_key) = self.config_read().embedding_key.clone() {
                info!(
                    "Using Cloudflare embedding API key ({} chars)",
                    api_key.len()
                );
                request_builder = request_builder.header("Authorization", format!("Bearer {}", api_key));
            } else {
                warn!("Cloudflare API key is NOT SET — embeddings will fail with 401");
            }

            request_builder
                .send()
                .await
                .context("Failed to send request to Cloudflare AI embedding service")?
        } else if is_hf_inference {
            // HuggingFace Inference API (old format): {"inputs": "text"}
            // Process one text at a time for HuggingFace Inference
            let mut all_embeddings = Vec::new();

            for text in &truncated_texts {
                let hf_request = serde_json::json!({
                    "inputs": text
                });

                let request_size = serde_json::to_string(&hf_request)
                    .map(|s| s.len())
                    .unwrap_or(0);
                trace!("Sending HuggingFace Inference request to {} (size: {} bytes)",
                      self.config_read().embedding_url.clone(), request_size);

                let mut request_builder = self.client
                    .post(&self.config_read().embedding_url.clone())
                    .json(&hf_request);

                // Add Authorization header if API key is provided
                if let Some(ref api_key) = self.config_read().embedding_key.clone() {
                    request_builder = request_builder.header("Authorization", format!("Bearer {}", api_key));
                }

                let resp = request_builder
                    .send()
                    .await
                    .context("Failed to send request to HuggingFace Inference embedding service")?;

                let status = resp.status();
                if !status.is_success() {
                    let error_bytes = resp.bytes().await.unwrap_or_default();
                    let error_text = String::from_utf8_lossy(&error_bytes[..error_bytes.len().min(1024)]);
                    return Err(anyhow::anyhow!(
                        "HuggingFace Inference embedding service error {}: {}",
                        status,
                        error_text
                    ));
                }

                let response_bytes = resp.bytes().await
                    .context("Failed to read HuggingFace Inference embedding response bytes")?;

                trace!("Received HuggingFace Inference response: {} bytes", response_bytes.len());

                if response_bytes.len() > 50 * 1024 * 1024 {
                    return Err(anyhow::anyhow!(
                        "Embedding response too large: {} bytes (max 50MB)",
                        response_bytes.len()
                    ));
                }

                // Parse HuggingFace Inference response (single array for single input)
                let embedding_vec: Vec<f32> = serde_json::from_slice(&response_bytes)
                    .with_context(|| {
                        let preview = std::str::from_utf8(&response_bytes)
                            .map(|s| botlib::text::truncate_at_char_boundary(s, 200))
                            .unwrap_or("<invalid utf8>");
                        format!("Failed to parse HuggingFace Inference embedding response. Preview: {}", preview)
                    })?;

                all_embeddings.push(Embedding {
                    vector: embedding_vec,
                    dimensions: self.config_read().dimensions,
                    model: self.config_read().embedding_model.clone(),
                    tokens_used: None,
                });
            }

            return Ok(all_embeddings);
        } else {
            // Standard embedding service format (OpenAI-compatible, Scaleway, llama.cpp, local server, etc.)
            // This includes Scaleway which uses OpenAI-compatible format: {"input": [texts], "model": "model-name"}
            let request = EmbeddingRequest {
                input: truncated_texts,
                model: self.config_read().embedding_model.clone(),
            };

            let request_size = serde_json::to_string(&request)
                .map(|s| s.len())
                .unwrap_or(0);

            // Log the API format being used
            if is_scaleway {
                trace!("Sending Scaleway (OpenAI-compatible) request to {} (size: {} bytes)",
                      self.config_read().embedding_url.clone(), request_size);
            } else {
                trace!("Sending standard embedding request to {} (size: {} bytes)",
                      self.config_read().embedding_url.clone(), request_size);
            }

            // Build request
            let mut request_builder = self.client
                .post(&self.config_read().embedding_url.clone())
                .json(&request);

            // Add Authorization header if API key is provided (for Scaleway, OpenAI, etc.)
            if let Some(ref api_key) = self.config_read().embedding_key.clone() {
                request_builder = request_builder.header("Authorization", format!("Bearer {}", api_key));
            }

            request_builder
                .send()
                .await
                .context("Failed to send request to embedding service")?
        };

        let status = response.status();
        if !status.is_success() {
            let error_bytes = response.bytes().await.unwrap_or_default();
            let error_text = String::from_utf8_lossy(&error_bytes[..error_bytes.len().min(1024)]);
            return Err(anyhow::anyhow!(
                "Embedding service error {}: {}",
                status,
                error_text
            ));
        }

        let response_bytes = response.bytes().await
            .context("Failed to read embedding response bytes")?;

        trace!("Received response: {} bytes", response_bytes.len());

        if response_bytes.len() > 50 * 1024 * 1024 {
            return Err(anyhow::anyhow!(
                "Embedding response too large: {} bytes (max 50MB)",
                response_bytes.len()
            ));
        }

        let embedding_response: EmbeddingResponse = serde_json::from_slice(&response_bytes)
            .with_context(|| {
                let preview = std::str::from_utf8(&response_bytes)
                    .map(|s| botlib::text::truncate_at_char_boundary(s, 200))
                    .unwrap_or("<invalid utf8>");
                format!("Failed to parse embedding response. Preview: {}", preview)
            })?;

        drop(response_bytes);

        let embeddings = match embedding_response {
            EmbeddingResponse::OpenAI(openai_response) => {
                let mut embeddings = Vec::with_capacity(openai_response.data.len());
                for data in openai_response.data {
                    embeddings.push(Embedding {
                        vector: data.embedding,
                        dimensions: self.config_read().dimensions,
                        model: openai_response.model.clone(),
                        tokens_used: openai_response.usage.as_ref().map(|u| u.total_tokens),
                    });
                }
                embeddings
            }
            EmbeddingResponse::LlamaCpp(llama_response) => {
                let mut embeddings = Vec::new();
                for item in llama_response {
                    for embedding_vec in item.embedding {
                        embeddings.push(Embedding {
                            vector: embedding_vec,
                            dimensions: self.config_read().dimensions,
                            model: self.config_read().embedding_model.clone(),
                            tokens_used: None,
                        });
                    }
                }
                embeddings
            }
            EmbeddingResponse::HuggingFace(hf_response) => {
                let mut embeddings = Vec::with_capacity(hf_response.len());
                for embedding_vec in hf_response {
                    embeddings.push(Embedding {
                        vector: embedding_vec,
                        dimensions: self.config_read().dimensions,
                        model: self.config_read().embedding_model.clone(),
                        tokens_used: None,
                    });
                }
                embeddings
            }
            EmbeddingResponse::Generic(generic_response) => {
                let mut embeddings = Vec::with_capacity(generic_response.embeddings.len());
                for embedding_vec in generic_response.embeddings {
                    embeddings.push(Embedding {
                        vector: embedding_vec,
                        dimensions: self.config_read().dimensions,
                        model: generic_response.model.clone().unwrap_or_else(|| self.config_read().embedding_model.clone()),
                        tokens_used: generic_response.usage.as_ref().map(|u| u.total_tokens),
                    });
                }
                embeddings
            }
            EmbeddingResponse::Scaleway(scaleway_response) => {
                let mut embeddings = Vec::with_capacity(scaleway_response.data.len());
                for data in scaleway_response.data {
                    embeddings.push(Embedding {
                        vector: data.embedding,
                        dimensions: self.config_read().dimensions,
                        model: scaleway_response.model.clone().unwrap_or_else(|| self.config_read().embedding_model.clone()),
                        tokens_used: scaleway_response.usage.as_ref().map(|u| u.total_tokens),
                    });
                }
                embeddings
            }
            EmbeddingResponse::Cloudflare(cf_response) => {
                if !cf_response.success {
                    let error_msg = cf_response.errors.first()
                        .map(|e| format!("{}: {}", e.code, e.message))
                        .unwrap_or_else(|| "Unknown Cloudflare error".to_string());
                    return Err(anyhow::anyhow!("Cloudflare AI error: {}", error_msg));
                }
                let mut embeddings = Vec::with_capacity(cf_response.result.data.len());
                for embedding_vec in cf_response.result.data {
                    embeddings.push(Embedding {
                        vector: embedding_vec,
                        dimensions: self.config_read().dimensions,
                        model: self.config_read().embedding_model.clone(),
                        tokens_used: cf_response.result.meta.as_ref().and_then(|m| {
                            m.cost_metric_value_1.map(|v| v as usize)
                        }),
                    });
                }
                embeddings
            }
        };

        Ok(embeddings)
    }
}
