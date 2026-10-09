use super::*;

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl: 3600,
            semantic_matching: true,
            similarity_threshold: 0.95,
            max_similarity_checks: 100,
            key_prefix: "llm_cache".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
