use super::*;

impl Default for HybridSearchConfig {
    fn default() -> Self {
        Self {
            dense_weight: 0.7,
            sparse_weight: 0.3,
            reranker_enabled: false,
            reranker_model: "cross-encoder/ms-marco-MiniLM-L-6-v2".to_string(),
            max_results: 10,
            min_score: 0.0,
            rrf_k: 60,
            bm25_enabled: true,
        }
    }
}
impl HybridSearchConfig {
    pub fn from_bot_config(_state: &(), bot_id: Uuid) -> Self {
        let mut config = Self::default();

        if let Some(vault_configs) = read_bot_config_from_vault(&bot_id) {
            for (config_key, config_value) in vault_configs {
                match config_key.as_str() {
                    "rag-dense-weight" => {
                        config.dense_weight = config_value.parse().unwrap_or(0.7);
                    }
                    "rag-sparse-weight" => {
                        config.sparse_weight = config_value.parse().unwrap_or(0.3);
                    }
                    "rag-reranker-enabled" => {
                        config.reranker_enabled = config_value.to_lowercase() == "true";
                    }
                    "rag-reranker-model" => {
                        config.reranker_model = config_value;
                    }
                    "rag-max-results" => {
                        config.max_results = config_value.parse().unwrap_or(10);
                    }
                    "rag-min-score" => {
                        config.min_score = config_value.parse().unwrap_or(0.0);
                    }
                    "rag-rrf-k" => {
                        config.rrf_k = config_value.parse().unwrap_or(60);
                    }
                    "bm25-enabled" => {
                        config.bm25_enabled = config_value.to_lowercase() == "true";
                    }
                    _ => {}
                }
            }
        }

        let total = config.dense_weight + config.sparse_weight;
        if total > 0.0 {
            config.dense_weight /= total;
            config.sparse_weight /= total;
        }

        debug!(
            "Loaded HybridSearchConfig: dense={}, sparse={}, bm25_enabled={}",
            config.dense_weight, config.sparse_weight, config.bm25_enabled
        );

        config
    }

    pub fn use_sparse_search(&self) -> bool {
        self.bm25_enabled && self.sparse_weight > 0.0
    }

    pub fn use_dense_search(&self) -> bool {
        self.dense_weight > 0.0
    }
}
