use super::*;

impl std::fmt::Debug for KbIndexer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KbIndexer")
            .field("document_processor", &self.document_processor)
            .field("embedding_generator", &self.embedding_generator)
            .field("qdrant_config", &self.qdrant_config)
            .field("http_client", &"reqwest::Client")
            .field("db_pool", &"DbPool")
            .finish()
    }
}
