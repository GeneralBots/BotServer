use super::*;

impl Default for QdrantConfig {
    fn default() -> Self {
        Self {
            url: "".to_string(),
            api_key: None,
            timeout_secs: 30,
        }
    }
}
impl QdrantConfig {
    pub fn from_config(pool: DbPool, bot_id: &Uuid) -> Self {
        let (url, api_key, _collection) = if let Ok(sm) = crate::shared::utils::get_secrets_manager_sync() {
            get_vectordb_config_from_value(&sm)
        } else {
            let config_manager = ConfigManager::new(pool.clone());
            let url = config_manager
                .get_config(bot_id, "vectordb-url", None)
                .unwrap_or_else(|_| "".to_string());
            let url = if url.is_empty() {
                std::env::var("VECTORDB_URL").unwrap_or_default()
            } else {
                url
            };
            (url, "".to_string(), "default".to_string())
        };
        Self {
            url,
            api_key: Some(api_key),
            timeout_secs: 30,
        }
    }
}
