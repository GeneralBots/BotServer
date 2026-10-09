use super::*;

impl CachedLLMProvider {
    async fn find_similar_cached(
        &self,
        prompt: &str,
        messages: &Value,
        model: &str,
        threshold: f32,
        bot_id: &str,
    ) -> Option<CachedResponse> {
        let embedding_service = self.embedding_service.as_ref()?;

        let actual_messages = if messages.get("messages").is_some() {
            messages.get("messages").unwrap_or(messages)
        } else {
            messages
        };

        // Extract ONLY the latest user question for semantic matching
        // This prevents false positives from matching on old conversation history
        let latest_user_question = if let Some(msgs) = actual_messages.as_array() {
            // Find the last message with role "user"
            msgs.iter()
                .rev()
                .find_map(|msg| {
                    if msg.get("role").and_then(|r| r.as_str()) == Some("user") {
                        msg.get("content").and_then(|c| c.as_str())
                    } else {
                        None
                    }
                })
                .unwrap_or("")
        } else {
            ""
        };

        // Use only the latest user question for semantic matching, not the full history
        // The prompt contains system context, so we combine with latest question
        let semantic_query = if latest_user_question.is_empty() {
            prompt.to_string()
        } else {
            format!("{}\n{}", prompt, latest_user_question)
        };

        // Debug: log the text being sent for embedding (truncated, no full user content)
        debug!(
            "Embedding request text (len={}, using latest user question, first {} chars)",
            semantic_query.len(),
            semantic_query.chars().take(80).count()
        );

        let prompt_embedding = match embedding_service.get_embedding(&semantic_query).await {
            Ok(emb) => emb,
            Err(e) => {
                debug!("Failed to get embedding for prompt: {}", e);
                return None;
            }
        };

        let mut conn = match self.cache.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                debug!("Failed to connect to cache for semantic search: {}", e);
                return None;
            }
        };

        let pattern = format!("{}:{}:{}:*", self.config.key_prefix, bot_id, model);
        let keys: Vec<String> = match conn.keys(pattern).await {
            Ok(k) => k,
            Err(e) => {
                debug!("Failed to get cache keys: {}", e);
                return None;
            }
        };

        let mut best_match: Option<(CachedResponse, f32)> = None;
        let check_limit = keys.len().min(self.config.max_similarity_checks);

        for key in keys.iter().take(check_limit) {
            if let Ok(cached_json) = conn.get::<_, String>(key).await {
                if let Ok(cached) = serde_json::from_str::<CachedResponse>(&cached_json) {
                    if let Some(ref cached_embedding) = cached.embedding {
                        let similarity = embedding_service
                            .compute_similarity(&prompt_embedding, cached_embedding)
                            .await;

                        if similarity >= threshold
                            && best_match.as_ref().is_none_or(|(_, s)| *s < similarity)
                        {
                            best_match = Some((cached.clone(), similarity));
                        }
                    }
                }
            }
        }

        if let Some((mut cached, similarity)) = best_match {
            debug!("Found semantic match with similarity: {}", similarity);

            cached.hit_count += 1;
            let cache_key =
                self.generate_cache_key(&cached.prompt, &cached.messages, &cached.model, bot_id);
            let _ = conn
                .set_ex::<_, _, ()>(
                    &cache_key,
                    serde_json::to_string(&cached).unwrap_or_default(),
                    self.config.ttl,
                )
                .await;
            return Some(cached);
        }

        None
    }

    async fn cache_response(&self, prompt: &str, messages: &Value, model: &str, response: &str, bot_id: &str) {
        let actual_messages = if messages.get("messages").is_some() {
            messages.get("messages").unwrap_or(messages)
        } else {
            messages
        };

        let cache_key = self.generate_cache_key(prompt, actual_messages, model, bot_id);

        let mut conn = match self.cache.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                debug!("Failed to connect to cache for storing: {}", e);
                return;
            }
        };

        let embedding = if let Some(ref service) = self.embedding_service {
            // Extract ONLY the latest user question for embedding
            // Same logic as find_similar_cached to ensure consistency
            let latest_user_question = if let Some(msgs) = actual_messages.as_array() {
                msgs.iter()
                    .rev()
                    .find_map(|msg| {
                        if msg.get("role").and_then(|r| r.as_str()) == Some("user") {
                            msg.get("content").and_then(|c| c.as_str())
                        } else {
                            None
                        }
                    })
                    .unwrap_or("")
            } else {
                ""
            };

            let semantic_query = if latest_user_question.is_empty() {
                prompt.to_string()
            } else {
                format!("{}\n{}", prompt, latest_user_question)
            };

            service.get_embedding(&semantic_query).await.ok()
        } else {
            None
        };

        let cached_response = CachedResponse {
            response: response.to_string(),
            prompt: prompt.to_string(),
            messages: actual_messages.clone(),
            model: model.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            hit_count: 0,
            embedding,
        };

        match serde_json::to_string(&cached_response) {
            Ok(json) => {
                if let Err(e) = conn
                    .set_ex::<_, _, ()>(&cache_key, json, self.config.ttl)
                    .await
                {
                    debug!("Failed to cache response: {}", e);
                } else {
                    trace!(
                        "Cached response for prompt: ~{} tokens",
                        estimate_token_count(prompt)
                    );
                }
            }
            Err(e) => {
                debug!("Failed to serialize cached response: {}", e);
            }
        }
    }

    pub async fn get_cache_stats(
        &self,
    ) -> Result<CacheStats, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.cache.get_multiplexed_async_connection().await?;

        let pattern = format!("{}:*", self.config.key_prefix);
        let keys: Vec<String> = conn.keys(pattern).await?;

        let mut total_hits = 0u32;
        let mut total_size = 0usize;
        let mut model_stats: std::collections::HashMap<String, u32> =
            std::collections::HashMap::new();

        for key in keys.iter() {
            if let Ok(cached_json) = conn.get::<_, String>(key).await {
                total_size += cached_json.len();
                if let Ok(cached) = serde_json::from_str::<CachedResponse>(&cached_json) {
                    total_hits += cached.hit_count;
                    *model_stats.entry(cached.model.clone()).or_insert(0) += 1;
                }
            }
        }

        Ok(CacheStats {
            total_entries: keys.len(),
            total_hits,
            total_size_bytes: total_size,
            model_distribution: model_stats,
        })
    }

    pub async fn clear_cache(
        &self,
        model: Option<&str>,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.cache.get_multiplexed_async_connection().await?;

        let pattern = if let Some(m) = model {
            format!("{}:*:{}:*", self.config.key_prefix, m)
        } else {
            format!("{}:*", self.config.key_prefix)
        };

        let keys: Vec<String> = conn.keys(pattern).await?;
        let count = keys.len();

        for key in keys {
            let _: Result<(), _> = conn.del(&key).await;
        }

        info!("Cleared {} cache entries", count);
        Ok(count)
    }
}
