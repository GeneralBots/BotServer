use super::*;

impl KbEmbeddingGenerator {
    /// Poison-recovering read lock: a panicked writer must not take the
    /// whole server down (#1368). The guard data is still usable.
    pub(super) fn config_read(&self) -> std::sync::RwLockReadGuard<'_, EmbeddingConfig> {
        self.config.read().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Poison-recovering write lock — same rationale as `config_read`.
    pub(super) fn config_write(&self) -> std::sync::RwLockWriteGuard<'_, EmbeddingConfig> {
        self.config.write().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn new(config: EmbeddingConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .connect_timeout(Duration::from_secs(config.connect_timeout_seconds))
            .pool_max_idle_per_host(2)
            .pool_idle_timeout(Duration::from_secs(30))
            .tcp_keepalive(Duration::from_secs(60))
            .tcp_nodelay(true)
            .build()
            .unwrap_or_else(|e| {
                info!("Failed to create HTTP client with timeout: {}, using default", e);
                Client::new()
            });

        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_requests));

        Self {
            config: std::sync::RwLock::new(config),
            client,
            semaphore,
        }
    }

    pub fn reload_config(&self, pool: &DbPool, bot_id: &uuid::Uuid) {
        let new_config = EmbeddingConfig::from_bot_config(pool, bot_id);
        *self.config_write() = new_config;
    }

    pub(super) fn extract_base_url(url: &str) -> String {
        if let Ok(parsed) = url::Url::parse(url) {
            format!(
                "{}://{}{}",
                parsed.scheme(),
                parsed.host_str().unwrap_or("localhost"),
                parsed.port().map(|p| format!(":{}", p)).unwrap_or_default()
            )
        } else {
            url.to_string()
        }
    }

    pub async fn check_health(&self) -> bool {
        // Remote HTTPS APIs (Cloudflare Workers AI, OpenAI, etc.) are assumed available
        // — they don't have /health endpoints and return 401/403/301 on probe.
        // Only local servers need TCP health checks.
        if self.config_read().embedding_url.clone().starts_with("https://") {
            info!("Embedding server is remote HTTPS API ({}), assuming available", self.config_read().embedding_url.clone());
            set_embedding_server_ready(true);
            return true;
        }

        // Strategy for local servers: try /health endpoint on BASE URL first.
        // - 200 OK → local server with health endpoint, ready
        // - 404/405 etc → server is reachable but has no /health (remote API or llama.cpp)
        // - Connection refused/timeout → server truly unavailable
        // Extract base URL (scheme://host:port) from embedding URL for health check
        let (embedding_url, connect_timeout) = {
            let cfg = self.config_read();
            (cfg.embedding_url.clone(), cfg.connect_timeout_seconds)
        };
        let base_url = Self::extract_base_url(&embedding_url);
        let health_url = format!("{}/health", base_url);

        match tokio::time::timeout(
            Duration::from_secs(connect_timeout),
            self.client.get(&health_url).send()
        ).await {
        Ok(Ok(response)) => {
            let status = response.status();
            if status.is_success() {
                info!("Embedding server health check passed ({})", self.config_read().embedding_url.clone());
                set_embedding_server_ready(true);
                true
            } else if status.as_u16() == 404 || status.as_u16() == 405 {
                // Server is reachable but has no /health endpoint (remote API, llama.cpp /embedding-only)
                // Try a HEAD request to the base URL to confirm it's up
                info!("No /health endpoint at {} (status {}), probing base URL", base_url, status);
                match tokio::time::timeout(
                    Duration::from_secs(connect_timeout),
                    self.client.head(&base_url).send()
                ).await {
                    Ok(Ok(_)) => {
                        info!("Embedding server reachable at {}, marking as ready", base_url);
                        set_embedding_server_ready(true);
                        true
                    }
                    Ok(Err(e)) => {
                        info!("Embedding server unreachable at {}: {}", base_url, e);
                        set_embedding_server_ready(false);
                        false
                    }
                    Err(_) => {
                        info!("Embedding server probe timed out for {}", base_url);
                        set_embedding_server_ready(false);
                        false
                    }
                }
            } else if status.is_redirection() || status.as_u16() == 401 || status.as_u16() == 403 {
                // Redirect (301/302) or auth-required (401/403) means the server IS reachable
                // This is typical for remote APIs like Cloudflare Workers AI
                info!("Embedding server reachable at {} (status {} indicates external API), marking as ready", base_url, status);
                set_embedding_server_ready(true);
                true
            } else {
                info!("Embedding server health check returned status {}", status);
                set_embedding_server_ready(false);
                false
            }
            }
            Ok(Err(e)) => {
                // Connection failed entirely — server not running or network issue
                info!("Embedding server connection failed for {}: {}", self.config_read().embedding_url.clone(), e);
                set_embedding_server_ready(false);
                false
            }
            Err(_) => {
                info!("Embedding server health check timed out for {}", self.config_read().embedding_url.clone());
                set_embedding_server_ready(false);
                false
            }
        }
    }

    pub async fn wait_for_server(&self, max_wait_secs: u64) -> bool {
        let start = std::time::Instant::now();
        let max_wait = Duration::from_secs(max_wait_secs);

        info!("Waiting for embedding server at {} (max {}s)...",
              self.config_read().embedding_url.clone(), max_wait_secs);

        while start.elapsed() < max_wait {
            if self.check_health().await {
                info!("Embedding server is ready after {:?}", start.elapsed());
                return true;
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        info!("Embedding server not available after {}s", max_wait_secs);
        false
    }
    /// Get the configured embedding dimensions
    pub fn get_dimensions(&self) -> usize {
        self.config_read().dimensions
    }

    pub async fn generate_embeddings(

        &self,
        chunks: &[TextChunk],
    ) -> Result<Vec<(TextChunk, Embedding)>> {
        if chunks.is_empty() {
            return Ok(Vec::new());
        }

        if !self.wait_for_server(30).await {
            return Err(anyhow::anyhow!(
                "Embedding server not available at {}. Skipping embedding generation.",
                self.config_read().embedding_url.clone()
            ));
        }

        let start_mem = MemoryStats::current();
        trace!("Generating embeddings for {} chunks, RSS={}",
              chunks.len(), MemoryStats::format_bytes(start_mem.rss_bytes));

        let batch_size = self.config_read().batch_size;
        let timeout_seconds = self.config_read().timeout_seconds;
        let mut results = Vec::with_capacity(chunks.len());
        let total_batches = chunks.len().div_ceil(batch_size);

        for (batch_num, batch) in chunks.chunks(batch_size).enumerate() {
            let batch_start = MemoryStats::current();
            trace!("Processing batch {}/{} ({} items), RSS={}",
                  batch_num + 1,
                  total_batches,
                  batch.len(),
                  MemoryStats::format_bytes(batch_start.rss_bytes));

            let batch_embeddings = match tokio::time::timeout(
                Duration::from_secs(timeout_seconds),
                self.generate_batch_embeddings(batch)
            ).await {
                Ok(Ok(embeddings)) => embeddings,
                Ok(Err(e)) => {
                    warn!("Batch {} failed: {}", batch_num + 1, e);
                    // Continue with next batch instead of breaking completely
                    continue;
                }
                Err(_) => {
                    warn!("Batch {} timed out after {}s",
                          batch_num + 1, self.config_read().timeout_seconds);
                    // Continue with next batch instead of breaking completely
                    continue;
                }
            };

            let batch_end = MemoryStats::current();
            let delta = batch_end.rss_bytes.saturating_sub(batch_start.rss_bytes);
            trace!("Batch {} complete: {} embeddings, RSS={} (delta={})",
                  batch_num + 1,
                  batch_embeddings.len(),
                  MemoryStats::format_bytes(batch_end.rss_bytes),
                  MemoryStats::format_bytes(delta));

            if delta > 100 * 1024 * 1024 {
                warn!("Excessive memory growth detected ({}), stopping early",
                      MemoryStats::format_bytes(delta));
                for (chunk, embedding) in batch.iter().zip(batch_embeddings.iter()) {
                    results.push((chunk.clone(), embedding.clone()));
                }
                break;
            }

            for (chunk, embedding) in batch.iter().zip(batch_embeddings.iter()) {
                results.push((chunk.clone(), embedding.clone()));
            }

            if batch_num + 1 < total_batches {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        let end_mem = MemoryStats::current();
        trace!("Generated {} embeddings, RSS={} (total delta={})",
              results.len(),
              MemoryStats::format_bytes(end_mem.rss_bytes),
              MemoryStats::format_bytes(end_mem.rss_bytes.saturating_sub(start_mem.rss_bytes)));
        log_jemalloc_stats();

        Ok(results)
    }

    async fn generate_batch_embeddings(&self, chunks: &[TextChunk]) -> Result<Vec<Embedding>> {
        let _permit = self.semaphore.acquire().await
            .map_err(|e| anyhow::anyhow!("Failed to acquire semaphore: {}", e))?;

        let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
        let total_chars: usize = texts.iter().map(|t| t.len()).sum();

        trace!("generate_batch_embeddings: {} texts, {} total chars",
              texts.len(), total_chars);

        match self.generate_local_embeddings(&texts).await {
            Ok(embeddings) => {
                trace!("Local embeddings succeeded: {} vectors", embeddings.len());
                Ok(embeddings)
            }
            Err(e) => {
                warn!("Local embedding service failed: {}", e);
                Err(e)
            }
        }
    }
}
