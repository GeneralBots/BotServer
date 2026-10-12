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
impl botlib::traits::LLMProvider for MockLLMProvider {
    fn generate(
        &self,
        _prompt: &str,
        _config: &serde_json::Value,
        _model: &str,
        _key: &str,
    ) -> botlib::traits::BoxFutureResult {
        let response = self.response.clone();
        Box::pin(async move { Ok(response) })
    }

    fn generate_simple(&self, _prompt: &str) -> botlib::traits::BoxFutureString {
        let response = self.response.clone();
        Box::pin(async move { Ok(response) })
    }

    fn generate_with_context(&self, _prompt: &str, _context: &str) -> botlib::traits::BoxFutureString {
        let response = self.response.clone();
        Box::pin(async move { Ok(response) })
    }

    fn generate_stream(
        &self,
        _prompt: &str,
        _config: &serde_json::Value,
        _tx: tokio::sync::mpsc::Sender<String>,
        _model: &str,
        _key: &str,
        _tools: Option<&Vec<serde_json::Value>>,
    ) -> botlib::traits::BoxFutureUnit {
        Box::pin(async move { Ok(()) })
    }
}

