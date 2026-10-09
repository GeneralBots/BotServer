use super::*;

#[async_trait]
impl LLMProvider for CachedLLMProvider {
    async fn generate(
        &self,
        prompt: &str,
        messages: &Value,
        model: &str,
        key: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let bot_id = messages
            .get("bot_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        if !self.is_cache_enabled(bot_id).await {
            trace!("Cache disabled for bot {}, bypassing", bot_id);
            return self.provider.generate(prompt, messages, model, key).await;
        }

        let bot_cache_config = self.get_bot_cache_config(bot_id);

        if let Some(cached) = self.get_cached_response(prompt, messages, model, bot_id).await {
            info!("Cache hit (exact match) for bot {}", bot_id);
            return Ok(cached.response);
        }

        if bot_cache_config.semantic_matching && self.embedding_service.is_some() {
            if let Some(cached) = self.find_similar_cached(prompt, messages, model, bot_cache_config.similarity_threshold, bot_id).await {
                info!(
                    "Cache hit (semantic match) for bot {} with similarity threshold {}",
                    bot_id, bot_cache_config.similarity_threshold
                );
                return Ok(cached.response);
            }
        }

        debug!("Cache miss for bot {}, generating new response", bot_id);
        let response = self.provider.generate(prompt, messages, model, key).await?;

        self.cache_response(prompt, messages, model, &response, bot_id)
            .await;

        Ok(response)
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
        let bot_id = messages
            .get("bot_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        if !self.is_cache_enabled(bot_id).await {
            trace!("Cache disabled for streaming, bypassing");
            return self
                .provider
                .generate_stream(prompt, messages, tx, model, key, tools)
                .await;
        }

        if let Some(cached) = self.get_cached_response(prompt, messages, model, bot_id).await {
            for chunk in cached.response.chars().collect::<Vec<_>>().chunks(50) {
                let chunk_str: String = chunk.iter().collect();
                if tx.send(chunk_str).await.is_err() {
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
            return Ok(());
        }

        let (buffer_tx, mut buffer_rx) = mpsc::channel::<String>(100);
        let tx_clone = tx.clone();
        let mut full_response = String::new();

        let forward_task = tokio::spawn(async move {
            while let Some(chunk) = buffer_rx.recv().await {
                full_response.push_str(&chunk);
                if tx_clone.send(chunk).await.is_err() {
                    break;
                }
            }
            full_response
        });

        self.provider
            .generate_stream(prompt, messages, buffer_tx, model, key, tools)
            .await?;

        let full_response = forward_task.await?;
        self.cache_response(prompt, messages, model, &full_response, bot_id)
            .await;

        Ok(())
    }

    async fn cancel_job(
        &self,
        session_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.provider.cancel_job(session_id).await
    }
}

#[derive(Debug)]
