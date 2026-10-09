use super::*;

impl KbIndexer {
    pub fn new(embedding_config: EmbeddingConfig, qdrant_config: QdrantConfig) -> Self {
        let document_processor = DocumentProcessor::default();
        let embedding_generator = KbEmbeddingGenerator::new(embedding_config);

        let http_client = create_tls_client(Some(qdrant_config.timeout_secs));

        Self {
            document_processor,
            embedding_generator,
            qdrant_config,
            http_client,
            db_pool: None,
        }
    }

    pub fn new_with_pool(embedding_config: EmbeddingConfig, qdrant_config: QdrantConfig, db_pool: DbPool) -> Self {
        let document_processor = DocumentProcessor::default();
        let embedding_generator = KbEmbeddingGenerator::new(embedding_config);

        let http_client = create_tls_client(Some(qdrant_config.timeout_secs));

        Self {
            document_processor,
            embedding_generator,
            qdrant_config,
            http_client,
            db_pool: Some(db_pool),
        }
    }

    pub fn get_db_pool(&self) -> Option<&DbPool> {
        self.db_pool.as_ref()
    }

    pub fn get_pool(&self) -> Result<DbPool, anyhow::Error> {
        self.db_pool
            .clone()
            .ok_or_else(|| anyhow::anyhow!("DbPool not available"))
    }

    pub async fn check_qdrant_health(&self) -> Result<bool> {
        let health_url = format!("{}/healthz", self.qdrant_config.url);

        match self.http_client.get(&health_url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    pub async fn index_kb_folder(
        &self,
        bot_id: Uuid,
        bot_name: &str,
        kb_name: &str,
        kb_path: &Path,
    ) -> Result<IndexingResult> {
        let start_mem = MemoryStats::current();
        info!("Indexing KB folder: {} for bot {} [START RSS={}]",
              kb_name, bot_name, MemoryStats::format_bytes(start_mem.rss_bytes));
        log_jemalloc_stats();

        // Reload embedding config from database to detect config.csv changes
        if let Some(ref pool) = self.db_pool {
            self.embedding_generator.reload_config(pool, &bot_id);
        }

        if !is_embedding_server_ready() {
            info!("Embedding server not ready yet, waiting up to 60s...");
            if !self.embedding_generator.wait_for_server(60).await {
                info!(
                    "Embedding server is not available. KB indexing skipped. \
                    Wait for the embedding server to start before indexing."
                );
                return Err(anyhow::anyhow!(
                    "Embedding server not available. KB indexing deferred until embedding service is ready."
                ));
            }
        }

        if !self.check_qdrant_health().await.unwrap_or(false) {
            info!(
                "Qdrant vector database is not available at {}. KB indexing skipped. \
                Install and start vector_db component to enable KB indexing.",
                self.qdrant_config.url
            );
            return Err(anyhow::anyhow!(
                "Qdrant vector database is not available at {}. Start the vector_db service to enable KB indexing.",
                self.qdrant_config.url
            ));
        }

        let bot_id_short = bot_id.to_string().chars().take(8).collect::<String>();
        let collection_name = format!("{}_{}_{}", bot_name, bot_id_short, kb_name);

        self.ensure_collection_exists(&collection_name).await?;

        let before_docs = MemoryStats::current();
        trace!("Before process_kb_folder RSS={}",
              MemoryStats::format_bytes(before_docs.rss_bytes));

        let documents = self.document_processor.process_kb_folder(kb_path).await?;

        let after_docs = MemoryStats::current();
        trace!("After process_kb_folder: {} documents, RSS={} (delta={})",
              documents.len(),
              MemoryStats::format_bytes(after_docs.rss_bytes),
              MemoryStats::format_bytes(after_docs.rss_bytes.saturating_sub(before_docs.rss_bytes)));

        let mut total_chunks = 0;
        let mut indexed_documents = 0;
        const BATCH_SIZE: usize = 5; // Smaller batch size to prevent memory exhaustion
        let mut batch_docs = Vec::with_capacity(BATCH_SIZE);

        // Process documents in iterator to avoid keeping all in memory
        let doc_iter = documents.into_iter();

        for (doc_path, chunks) in doc_iter {
            if chunks.is_empty() {
                debug!("Skipping document with no chunks: {}", doc_path);
                continue;
            }

            batch_docs.push((doc_path, chunks));

            // Process batch when full
            if batch_docs.len() >= BATCH_SIZE {
                let (processed, chunks_count) = self.process_document_batch(&collection_name, &mut batch_docs).await?;
                indexed_documents += processed;
                total_chunks += chunks_count;

                // Clear batch and force memory cleanup
                batch_docs.clear();
                batch_docs.shrink_to_fit();

                // Yield control to prevent blocking
                tokio::task::yield_now().await;

                // Memory pressure check - more aggressive
                let current_mem = MemoryStats::current();
                if current_mem.rss_bytes > 1_500_000_000 { // 1.5GB threshold (reduced)
                    warn!("High memory usage detected: {}, forcing cleanup",
                          MemoryStats::format_bytes(current_mem.rss_bytes));

                    // Force garbage collection hint
                    std::hint::black_box(&batch_docs);

                    // Add delay to allow memory cleanup
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }

        // Process remaining documents in final batch
        if !batch_docs.is_empty() {
            let (processed, chunks_count) = self.process_document_batch(&collection_name, &mut batch_docs).await?;
            indexed_documents += processed;
            total_chunks += chunks_count;
        }

        self.update_collection_metadata(&collection_name, bot_name, kb_name, total_chunks)?;

        let end_mem = MemoryStats::current();
        trace!("Indexing complete: {} docs, {} chunks, RSS={} (total delta={})",
              indexed_documents, total_chunks,
              MemoryStats::format_bytes(end_mem.rss_bytes),
              MemoryStats::format_bytes(end_mem.rss_bytes.saturating_sub(start_mem.rss_bytes)));
        log_jemalloc_stats();

        Ok(IndexingResult {
            collection_name,
            documents_processed: indexed_documents,
            chunks_indexed: total_chunks,
        })
    }

    async fn process_document_batch(
        &self,
        collection_name: &str,
        batch_docs: &mut Vec<(String, Vec<TextChunk>)>,
    ) -> Result<(usize, usize)> {
        let mut processed_count = 0;
        let mut total_chunks = 0;

        // Process documents one by one to minimize memory usage
        while let Some((doc_path, chunks)) = batch_docs.pop() {
            let before_embed = MemoryStats::current();
            trace!(
                "Processing document: {} ({} chunks) RSS={}",
                doc_path,
                chunks.len(),
                MemoryStats::format_bytes(before_embed.rss_bytes)
            );

            // Re-validate embedding server is still available
            if !is_embedding_server_ready() {
                info!("Embedding server became unavailable during indexing, aborting batch");
                return Err(anyhow::anyhow!(
                    "Embedding server became unavailable during KB indexing. Processed {} documents before failure.",
                    processed_count
                ));
            }

            // Process chunks in smaller sub-batches to prevent memory exhaustion
            const CHUNK_BATCH_SIZE: usize = 20; // Process 20 chunks at a time
            let chunk_batches = chunks.chunks(CHUNK_BATCH_SIZE);

            for chunk_batch in chunk_batches {
                trace!("Processing chunk batch of {} chunks", chunk_batch.len());

                let embeddings = match self
                    .embedding_generator
                    .generate_embeddings(chunk_batch)
                    .await
                {
                    Ok(emb) => emb,
                    Err(e) => {
                        warn!("Embedding generation failed for {}: {}", doc_path, e);
                        break; // Skip to next document
                    }
                };

                let points = Self::create_qdrant_points(&doc_path, embeddings)?;
                self.upsert_points(collection_name, points).await?;

                // Yield control between chunk batches
                tokio::task::yield_now().await;
            }

            let after_embed = MemoryStats::current();
            trace!("After processing document: RSS={} (delta={})",
                  MemoryStats::format_bytes(after_embed.rss_bytes),
                  MemoryStats::format_bytes(after_embed.rss_bytes.saturating_sub(before_embed.rss_bytes)));

            total_chunks += chunks.len();
            processed_count += 1;

            // Force memory cleanup after each document
            std::hint::black_box(&chunks);
        }

        Ok((processed_count, total_chunks))
    }
}
