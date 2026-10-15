//! llm_setup - extracted from bootstrap.rs

use botcore::config::ConfigManager;
use log::info;
use std::sync::Arc;
use uuid::Uuid;


pub(crate) fn init_llm_provider(
    config_manager: &ConfigManager,
    default_bot_id: &str,
    dynamic_llm_provider: Arc<crate::llm::dynamic_provider::DynamicLLMProvider>,
    pool: &botcore::shared::utils::DbPool,
    redis_client: Option<Arc<redis::Client>>,
) -> Arc<dyn crate::llm::traits::LLMProvider> {
    use crate::llm::cache::{CacheConfig, CachedLLMProvider, EmbeddingService, LocalEmbeddingService};

    if let Some(ref cache) = redis_client {
        let bot_id = Uuid::parse_str(default_bot_id).unwrap_or_default();
        let embedding_url = config_manager
            .get_config(
                &bot_id,
                "embedding-url",
                Some(""),
            )
            .unwrap_or_else(|_| "".to_string());
        let embedding_model = config_manager
            .get_config(&bot_id, "embedding-model", Some("all-MiniLM-L6-v2"))
            .unwrap_or_else(|_| "all-MiniLM-L6-v2".to_string());
        let embedding_key = config_manager
            .get_config(&bot_id, "embedding-key", None)
            .ok();
        let semantic_cache_enabled = config_manager
            .get_config(&bot_id, "llm-cache-semantic", Some("false"))
            .unwrap_or_else(|_| "false".to_string())
            .to_lowercase() == "true";

        let similarity_threshold = config_manager
            .get_config(&bot_id, "llm-cache-threshold", Some("0.85"))
            .unwrap_or_else(|_| "0.85".to_string())
            .parse::<f32>()
            .unwrap_or(0.85);

        info!("Embedding URL: {}", embedding_url);
        info!("Embedding Model: {}", embedding_model);
        info!("Embedding Key: {}", if embedding_key.is_some() { "configured" } else { "not set" });
        info!("Semantic Cache Enabled: {}", semantic_cache_enabled);
        info!("Cache Similarity Threshold: {}", similarity_threshold);

        let embedding_service = if semantic_cache_enabled {
            Some(Arc::new(LocalEmbeddingService::new(
                embedding_url,
                embedding_model,
                embedding_key,
            )) as Arc<dyn EmbeddingService>)
        } else {
            None
        };

        let cache_config = CacheConfig {
            ttl: 3600,
            semantic_matching: semantic_cache_enabled,
            similarity_threshold,
            max_similarity_checks: 100,
            key_prefix: "llm_cache".to_string(),
        };

        Arc::new(CachedLLMProvider::with_db_pool(
            dynamic_llm_provider.clone() as Arc<dyn crate::llm::traits::LLMProvider>,
            cache.clone(),
            cache_config,
            embedding_service,
            pool.clone(),
        ))
    } else {
        dynamic_llm_provider.clone() as Arc<dyn crate::llm::traits::LLMProvider>
    }
}
