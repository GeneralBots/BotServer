use std::sync::Arc;
pub use botllm::*;


pub struct BotlibLLMProviderWrapper {
    pub inner: Arc<dyn crate::llm::LLMProvider>,
    pub model: String,
    pub key: String,
}

impl std::fmt::Debug for BotlibLLMProviderWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BotlibLLMProviderWrapper").finish_non_exhaustive()
    }
}

impl BotlibLLMProviderWrapper {
    pub fn new(inner: Arc<dyn crate::llm::LLMProvider>, model: String, key: String) -> Self {
        Self { inner, model, key }
    }
}

/// Documented adapter (#1498): binds a provider to one bot's model and key
/// so callers can use `generate_simple` / `generate_with_context` without
/// repeating the configuration on every call. The provider contract itself
/// is `botlib::traits::LLMProvider` — this is not a second trait.
#[async_trait::async_trait]
impl botlib::traits::LLMProvider for BotlibLLMProviderWrapper {
    async fn generate(
        &self,
        prompt: &str,
        config: &serde_json::Value,
        model: &str,
        key: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        self.inner
            .generate(prompt, config, model, key)
            .await
            .map_err(|e| Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn generate_simple(&self, prompt: &str) -> Result<String, String> {
        self.inner
            .generate(prompt, &serde_json::Value::Null, &self.model, &self.key)
            .await
            .map_err(|e| e.to_string())
    }

    async fn generate_with_context(&self, prompt: &str, context: &str) -> Result<String, String> {
        self.inner
            .generate(
                &format!("{context}\n\n{prompt}"),
                &serde_json::Value::Null,
                &self.model,
                &self.key,
            )
            .await
            .map_err(|e| e.to_string())
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        config: &serde_json::Value,
        tx: tokio::sync::mpsc::Sender<String>,
        model: &str,
        key: &str,
        tools: Option<&Vec<serde_json::Value>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.inner
            .generate_stream(prompt, config, tx, model, key, tools)
            .await
            .map_err(|e| Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn cancel_job(
        &self,
        session_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.inner.cancel_job(session_id).await
    }
}