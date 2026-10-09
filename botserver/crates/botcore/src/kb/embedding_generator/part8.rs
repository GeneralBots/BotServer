use super::*;

impl std::fmt::Debug for EmailEmbeddingGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmailEmbeddingGenerator")
            .field("generator", &self.generator)
            .finish()
    }
}
impl EmailEmbeddingGenerator {
    pub fn new(llm_endpoint: String) -> Self {
        Self {
            generator: EmbeddingGenerator::new(llm_endpoint),
        }
    }

    pub async fn generate_embedding(&self, email: &impl EmailLike) -> Result<Vec<f32>> {
        let text = format!(
            "Subject: {}\nFrom: {}\nTo: {}\n\n{}",
            email.subject(),
            email.from(),
            email.to(),
            email.body()
        );

        self.generator.generate_text_embedding(&text).await
    }

    pub async fn generate_text_embedding(&self, text: &str) -> Result<Vec<f32>> {
        self.generator.generate_text_embedding(text).await
    }
}
