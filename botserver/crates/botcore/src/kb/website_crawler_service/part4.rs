use super::*;

pub async fn ensure_crawler_service_running(
    state: Arc<AppState>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(_kb_manager) = &state.kb_manager {
        let service = Arc::new(WebsiteCrawlerService::new(
            state.conn.clone(),
        ));

        drop(service.start());

        trace!("Website crawler service initialized");

        Ok(())
    } else {
        info!("KB manager not available, website crawler service not started");
        Ok(())
    }
}
/// Build an `EmbeddingConfig` for a bot, reading most settings from the DB
/// but **overriding the API key from Vault** (per-bot path `gbo/{bot_name}` → `embedding-key`)
/// when available. Falls back transparently to the DB value when Vault is unavailable.
pub(super) async fn embedding_config_with_vault(
    pool: &DbPool,
    bot_id: &uuid::Uuid,
    bot_name: &str,
) -> EmbeddingConfig {
    // Start from the DB-backed config (URL, model, dimensions, etc.)
    let mut config = EmbeddingConfig::from_bot_config(pool, bot_id);

    // Try to upgrade the API key from Vault using the per-bot secret path.
    if let Ok(key) = std::env::var("EMBEDDING_KEY") {
        if !key.is_empty() {
            trace!("Loaded embedding key from environment for bot {}", bot_name);
            config.embedding_key = Some(key);
        }
    }

    config
}
