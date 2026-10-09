use super::*;

impl Default for EmbeddingConfig {
pub(super) fn default() -> Self {
    Self {
        embedding_url: "".to_string(),
        embedding_model: "BAAI/bge-multilingual-gemma2".to_string(),
        embedding_key: None,
        dimensions: 384, // Default to BGE-Small dimensions, will be overridden by config
        batch_size: 2,
        timeout_seconds: 60,
        max_concurrent_requests: 1,
        connect_timeout_seconds: 10,
    }
}

}
impl EmbeddingConfig {
    pub fn from_env() -> Self {
        Self::default()
    }

    pub fn from_bot_config(pool: &DbPool, _bot_id: &uuid::Uuid) -> Self {
        use crate::config::ConfigManager;

        let config_manager = ConfigManager::new(pool.clone());

        let embedding_url_raw = config_manager
            .get_config(_bot_id, "embedding-url", Some(""))
            .unwrap_or_default();

        let embedding_url = if embedding_url_raw.is_empty() {
            let default_url = "http://localhost:8082/v1/embeddings".to_string();
            info!("No embedding-url configured for bot {_bot_id}, using default: {default_url}");
            default_url
        } else {
            embedding_url_raw
        };

        let embedding_model = config_manager
            .get_config(_bot_id, "embedding-model", Some("BAAI/bge-multilingual-gemma2"))
            .unwrap_or_else(|_| "BAAI/bge-multilingual-gemma2".to_string());

        let embedding_key = config_manager
            .get_config(_bot_id, "embedding-key", Some(""))
            .ok()
            .filter(|s| !s.is_empty())
            // If value looks encrypted (1:nonce:ciphertext) but decryption failed,
            // it means the master key is ephemeral (dev mode) - reject it
            .filter(|s| !s.starts_with("1:"));

        let dimensions = config_manager
            .get_config(_bot_id, "embedding-dimensions", Some(""))
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or_else(|| Self::detect_dimensions(&embedding_model));

        info!("EmbeddingConfig::from_bot_config - bot_id: {}, embedding_url: {}, embedding_key: {}, dimensions: {}",
              _bot_id,
              if embedding_url.len() > 50 { &embedding_url[..50] } else { &embedding_url },
              if embedding_key.is_some() { "SET" } else { "NONE" },
              dimensions);

        let batch_size = config_manager
            .get_config(_bot_id, "embedding-batch-size", Some("16"))
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(16);

        let timeout_seconds = config_manager
            .get_config(_bot_id, "embedding-timeout", Some("60"))
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60);

        let max_concurrent_requests = config_manager
            .get_config(_bot_id, "embedding-concurrent", Some("1"))
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);

        Self {
            embedding_url,
            embedding_model,
            embedding_key,
            dimensions,
            batch_size,
            timeout_seconds,
            max_concurrent_requests,
            connect_timeout_seconds: 10,
        }
    }

    pub(super) fn detect_dimensions(model: &str) -> usize {
        if model.contains("gemma") || model.contains("Gemma") {
            2048
        } else if model.contains("small") || model.contains("MiniLM") {
            384
        } else if model.contains("base") || model.contains("mpnet") {
            768
        } else if model.contains("large") || model.contains("ada") {
            1536
        } else {
            384
        }
    }
}
