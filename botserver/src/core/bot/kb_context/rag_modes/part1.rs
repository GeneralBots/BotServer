use super::*;

impl RagMode {
    pub fn from_label(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "hybrid" => Self::Hybrid,
            "corrective" => Self::Corrective,
            "graph" => Self::Graph,
            "agentic" => Self::Agentic,
            "multimodal" => Self::Multimodal,
            _ => Self::Standard,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Hybrid => "hybrid",
            Self::Corrective => "corrective",
            Self::Graph => "graph",
            Self::Agentic => "agentic",
            Self::Multimodal => "multimodal",
        }
    }

    pub fn variants() -> &'static [&'static str] {
        &["standard", "hybrid", "corrective", "graph", "agentic", "multimodal"]
    }
}
pub(super) async fn create_llm_for_bot(bot_id: Uuid, db_pool: &DbPool) -> Option<Arc<dyn botlib::traits::LLMProvider>> {
    let cfg = ConfigManager::new(db_pool.clone());
    let llm_url = match cfg.get_config(&bot_id, "llm-url", Some("")) {
        Ok(url) if !url.is_empty() => url,
        _ => return None,
    };
    let llm_key = cfg.get_config(&bot_id, "llm-key", Some("")).unwrap_or_default();
    let llm_model = cfg.get_config(&bot_id, "llm-model", Some("")).unwrap_or_default();
    // Empty default: let create_llm_provider_from_url derive the endpoint from
    // the URL (a forced /v1/chat/completions breaks full chat-completions URLs).
    let endpoint_path = cfg.get_config(&bot_id, "llm-endpoint-path", Some("")).unwrap_or_default();
    let provider = crate::llm::provider_factory::create_llm_provider_from_url(
        &llm_url,
        if llm_model.is_empty() { None } else { Some(llm_model.clone()) },
        Some(endpoint_path),
        None,
    );
    Some(Arc::new(crate::llm::BotlibLLMProviderWrapper::new(provider, llm_model, llm_key)))
}
pub async fn search_by_mode(
    mode: RagMode,
    collection: &str,
    query: &str,
    limit: usize,
    bot_id: Uuid,
    db_pool: &DbPool,
) -> Vec<KbSearchResult> {
    match mode {
        RagMode::Standard => kb_context_search::search_qdrant(
            collection, query, limit, bot_id, db_pool,
        )
        .await
        .unwrap_or_default(),

        RagMode::Hybrid => hybrid_search(collection, query, limit, bot_id, db_pool).await,

        RagMode::Corrective => corrective_search(collection, query, limit, bot_id, db_pool).await,

        RagMode::Graph => graph_search(collection, query, limit, bot_id, db_pool).await,

        RagMode::Agentic => agentic_search(collection, query, limit, bot_id, db_pool).await,

        RagMode::Multimodal => multimodal_search(collection, query, limit, bot_id, db_pool).await,
    }
}
