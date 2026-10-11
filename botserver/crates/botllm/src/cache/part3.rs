use super::*;

impl CachedLLMProvider {
    pub fn new(
        provider: Arc<dyn LLMProvider>,
        cache: Arc<redis::Client>,
        config: CacheConfig,
        embedding_service: Option<Arc<dyn EmbeddingService>>,
    ) -> Self {
        info!("Initializing CachedLLMProvider with semantic cache");
        info!(
            "Cache config: TTL={}s, Semantic={}, Threshold={}",
            config.ttl, config.semantic_matching, config.similarity_threshold
        );

        Self {
            provider,
            cache,
            config,
            embedding_service,
            db_pool: None,
        }
    }

    pub fn with_db_pool(
        provider: Arc<dyn LLMProvider>,
        cache: Arc<redis::Client>,
        config: CacheConfig,
        embedding_service: Option<Arc<dyn EmbeddingService>>,
        db_pool: DbPool,
    ) -> Self {
        info!("Initializing CachedLLMProvider with semantic cache and DB pool");
        info!(
            "Cache config: TTL={}s, Semantic={}, Threshold={}",
            config.ttl, config.semantic_matching, config.similarity_threshold
        );

        Self {
            provider,
            cache,
            config,
            embedding_service,
            db_pool: Some(db_pool),
        }
    }

    pub(super) fn generate_cache_key(&self, prompt: &str, messages: &Value, model: &str, bot_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(prompt.as_bytes());
        hasher.update(messages.to_string().as_bytes());
        hasher.update(model.as_bytes());
        let hash = hasher.finalize();
        format!("{}:{}:{}:{}", self.config.key_prefix, bot_id, model, hex::encode(hash))
    }

    pub(crate) async fn is_cache_enabled(&self, bot_id: &str) -> bool {
        if let Some(ref db_pool) = self.db_pool {
            let bot_uuid = match Uuid::parse_str(bot_id) {
                Ok(uuid) => uuid,
                Err(_) => {
                    if bot_id == "default" {
                        Uuid::nil()
                    } else {
                        return self.config.semantic_matching;
                    }
                }
            };

            let config_manager = ConfigManager::new(db_pool.clone());
            let cache_enabled = config_manager
                .get_config(&bot_uuid, "llm-cache", Some("true"))
                .unwrap_or_else(|_| "true".to_string());

            return cache_enabled.to_lowercase() == "true";
        }

        let mut conn = match self.cache.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                debug!("Cache connection failed: {}", e);
                return self.config.semantic_matching;
            }
        };

        let config_key = format!("bot_config:{}:llm-cache", bot_id);
        match conn.get::<_, String>(config_key).await {
            Ok(value) => value.to_lowercase() == "true",
            Err(_) => self.config.semantic_matching,
        }
    }

    pub(super) fn get_bot_cache_config(&self, bot_id: &str) -> CacheConfig {
        if let Some(ref db_pool) = self.db_pool {
            let bot_uuid = match Uuid::parse_str(bot_id) {
                Ok(uuid) => uuid,
                Err(_) => {
                    if bot_id == "default" {
                        Uuid::nil()
                    } else {
                        return self.config.clone();
                    }
                }
            };

            let config_manager = ConfigManager::new(db_pool.clone());

            let ttl = config_manager
                .get_config(
                    &bot_uuid,
                    "llm-cache-ttl",
                    Some(&self.config.ttl.to_string()),
                )
                .unwrap_or_else(|_| self.config.ttl.to_string())
                .parse()
                .unwrap_or(self.config.ttl);

            let semantic_enabled = config_manager
                .get_config(&bot_uuid, "llm-cache-semantic", Some("false"))
                .unwrap_or_else(|_| "false".to_string())
                .to_lowercase()
                == "true";

            let threshold = config_manager
                .get_config(
                    &bot_uuid,
                    "llm-cache-threshold",
                    Some(&self.config.similarity_threshold.to_string()),
                )
                .unwrap_or_else(|_| self.config.similarity_threshold.to_string())
                .parse()
                .unwrap_or(self.config.similarity_threshold);

            CacheConfig {
                ttl,
                semantic_matching: semantic_enabled,
                similarity_threshold: threshold,
                max_similarity_checks: self.config.max_similarity_checks,
                key_prefix: self.config.key_prefix.clone(),
            }
        } else {
            self.config.clone()
        }
    }

    pub(crate) async fn get_cached_response(
        &self,
        prompt: &str,
        messages: &Value,
        model: &str,
        bot_id: &str,
    ) -> Option<CachedResponse> {
        let mut conn = match self.cache.get_multiplexed_async_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                debug!("Failed to connect to cache: {}", e);
                return None;
            }
        };

        let actual_messages = if messages.get("messages").is_some() {
            messages.get("messages").unwrap_or(messages)
        } else {
            messages
        };

        let cache_key = self.generate_cache_key(prompt, actual_messages, model, bot_id);

        if let Ok(cached_json) = conn.get::<_, String>(&cache_key).await {
            if let Ok(mut cached) = serde_json::from_str::<CachedResponse>(&cached_json) {
                cached.hit_count += 1;
                let _ = conn
                    .set_ex::<_, _, ()>(
                        &cache_key,
                        serde_json::to_string(&cached).unwrap_or_default(),
                        self.config.ttl,
                    )
                    .await;

                info!(
                    "Cache hit (exact match) for prompt: ~{} tokens",
                    estimate_token_count(prompt)
                );
                return Some(cached);
            }
        }

        if self.config.semantic_matching && self.embedding_service.is_some() {
            if let Some(similar) = self.find_similar_cached(prompt, messages, model, self.config.similarity_threshold, bot_id).await {
                info!(
                    "Cache hit (semantic match) for prompt: ~{} tokens",
                    estimate_token_count(prompt)
                );
                return Some(similar);
            }
        }

        debug!(
            "Cache miss for prompt: ~{} tokens",
            estimate_token_count(prompt)
        );
        None
    }
}
