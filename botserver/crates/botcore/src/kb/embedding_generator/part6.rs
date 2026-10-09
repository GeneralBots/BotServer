use super::*;

impl KbEmbeddingGenerator {
    pub async fn generate_single_embedding(&self, text: &str) -> Result<Embedding> {
        if !self.check_health().await {
            return Err(anyhow::anyhow!(
                "Embedding server not available at {}",
                self.config_read().embedding_url.clone()
            ));
        }

        let embeddings = self
            .generate_batch_embeddings(&[TextChunk {
                content: text.to_string(),
                metadata: super::document_processor::ChunkMetadata {
                    document_path: "query".to_string(),
                    document_title: None,
                    chunk_index: 0,
                    total_chunks: 1,
                    start_char: 0,
                    end_char: text.len(),
                    page_number: None,
                },
            }])
            .await?;

        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No embedding generated"))
    }
}
