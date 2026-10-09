use super::*;

impl std::fmt::Debug for CachedLLMProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedLLMProvider")
            .field("provider", &"<dyn LLMProvider>")
            .field("cache", &self.cache)
            .field("config", &self.config)
            .field("embedding_service", &self.embedding_service.is_some())
            .field("db_pool", &self.db_pool.is_some())
            .finish()
    }
}
