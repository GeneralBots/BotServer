use super::*;

impl std::fmt::Debug for EmbeddingGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddingGenerator")
            .field("kb_generator", &self.kb_generator)
            .finish()
    }
}
impl EmbeddingGenerator {
    pub fn new(llm_endpoint: String) -> Self {
        let config = EmbeddingConfig {
            embedding_url: llm_endpoint,
            ..Default::default()
        };

        Self {
            kb_generator: KbEmbeddingGenerator::new(config),
        }
    }

    pub async fn generate_text_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let embedding = self.kb_generator.generate_single_embedding(text).await?;
        Ok(embedding.vector)
    }

    /// Check if the embedding server is healthy
    pub async fn check_health(&self) -> bool {
        self.kb_generator.check_health().await
    }
}
