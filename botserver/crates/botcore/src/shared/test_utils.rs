#[cfg(feature = "llm")]
#[derive(Debug, Clone)]
pub struct MockLLMProvider {
    pub response: String,
}

#[cfg(feature = "llm")]
impl MockLLMProvider {
    pub fn new() -> Self {
        Self { response: String::new() }
    }
}

#[cfg(feature = "llm")]
#[async_trait::async_trait]
impl botlib::traits::LLMProvider for MockLLMProvider {
    async fn generate(
        &self,
        _prompt: &str,
        _config: &serde_json::Value,
        _model: &str,
        _key: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.response.clone())
    }

    async fn generate_simple(&self, _prompt: &str) -> Result<String, String> {
        Ok(self.response.clone())
    }

    async fn generate_with_context(
        &self,
        _prompt: &str,
        _context: &str,
    ) -> Result<String, String> {
        Ok(self.response.clone())
    }

    async fn generate_stream(
        &self,
        _prompt: &str,
        _config: &serde_json::Value,
        _tx: tokio::sync::mpsc::Sender<String>,
        _model: &str,
        _key: &str,
        _tools: Option<&Vec<serde_json::Value>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }
}

