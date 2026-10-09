use super::*;

impl std::fmt::Debug for KbEmbeddingGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KbEmbeddingGenerator")
            .field("config", &self.config_read())
            .field("client", &"Client")
            .field("semaphore", &"Semaphore")
            .finish()
    }
}
