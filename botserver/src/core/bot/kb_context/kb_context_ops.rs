use diesel::prelude::*;
use log::{debug, info, warn};
use uuid::Uuid;

use botcore::shared::utils::DbPool;

use super::{
    KbContext, RagMode, SessionKbAssociation, SessionWebsiteAssociation,
};

pub fn get_active_kbs(db_pool: &DbPool, session_id: Uuid) -> Vec<SessionKbAssociation> {
    let mut conn = match db_pool.get() {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to get DB connection for KB lookup: {}", e);
            return Vec::new();
        }
    };

    #[derive(QueryableByName)]
    struct KbAssocRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        kb_name: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        qdrant_collection: String,
        #[diesel(sql_type = diesel::sql_types::Bool)]
        is_active: bool,
    }

    let query = diesel::sql_query(
        "SELECT kb_name, qdrant_collection, is_active
        FROM session_kb_associations
        WHERE session_id = $1 AND is_active = true",
    )
    .bind::<diesel::sql_types::Uuid, _>(session_id);

    match query.load::<KbAssocRow>(&mut conn) {
        Ok(rows) => rows
            .into_iter()
            .map(|r| SessionKbAssociation {
                kb_name: r.kb_name,
                qdrant_collection: r.qdrant_collection,
                is_active: r.is_active,
            })
            .collect(),
        Err(e) => {
            debug!("No active KBs for session {}: {}", session_id, e);
            Vec::new()
        }
    }
}

pub fn get_active_websites(db_pool: &DbPool, session_id: Uuid) -> Vec<SessionWebsiteAssociation> {
    let mut conn = match db_pool.get() {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to get DB connection for website lookup: {}", e);
            return Vec::new();
        }
    };

    #[derive(QueryableByName)]
    struct WebsiteAssocRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        website_url: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        collection_name: String,
        #[diesel(sql_type = diesel::sql_types::Bool)]
        is_active: bool,
    }

    let query = diesel::sql_query(
        "SELECT website_url, collection_name, is_active
        FROM session_website_associations
        WHERE session_id = $1 AND is_active = true",
    )
    .bind::<diesel::sql_types::Uuid, _>(session_id);

    match query.load::<WebsiteAssocRow>(&mut conn) {
        Ok(rows) => rows
            .into_iter()
            .map(|r| SessionWebsiteAssociation {
                website_url: r.website_url,
                collection_name: r.collection_name,
                is_active: r.is_active,
            })
            .collect(),
        Err(e) => {
            debug!("No active websites for session {}: {}", session_id, e);
            Vec::new()
        }
    }
}

fn build_context_string(kb_contexts: &[KbContext]) -> String {
    if kb_contexts.is_empty() {
        return String::new();
    }

    let mut parts = vec!["\n--- Informações de Contexto (Base de Conhecimento) ---".to_string()];

    for ctx in kb_contexts {
        if ctx.search_results.is_empty() {
            continue;
        }

        parts.push(format!("\n## De '{}':", ctx.kb_name));

        for (idx, result) in ctx.search_results.iter().enumerate() {
            parts.push(format!(
                "\n### Resultado {} (relevância: {:.2}):\n{}",
                idx + 1,
                result.score,
                result.content
            ));
            if !result.document_path.is_empty() {
                parts.push(format!("Fonte: {}", result.document_path));
            }
        }
    }

    parts.push("\n--- Fim do Contexto ---\n".to_string());
    parts.join("\n")
}

fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

fn truncate_text(text: &str, max_tokens: usize) -> String {
    let mut tokens = 0usize;
    let mut result = String::new();
    for line in text.lines() {
        let line_tokens = estimate_tokens(line) + 1;
        if tokens + line_tokens > max_tokens {
            break;
        }
        tokens += line_tokens;
        result.push_str(line);
        result.push('\n');
    }
    result
}

pub async fn inject_kb_context(
    db_pool: &DbPool,
    session_id: Uuid,
    bot_id: Uuid,
    user_query: &str,
    messages: &mut serde_json::Value,
    max_context_tokens: usize,
) {
    use botcore::config::ConfigManager;
    use crate::basic::keywords::mention_config::MentionConfig;

    let cfg = ConfigManager::new(db_pool.clone());

    let mention_class_str = cfg.get_config(&bot_id, "mention-class", Some("all"))
        .unwrap_or_else(|_| "all".to_string());

    let mention_config = MentionConfig::from_string(&mention_class_str);

    let rag_mode_str = cfg.get_config(&bot_id, "rag-mode", Some("standard"))
        .unwrap_or_else(|_| "standard".to_string());
    let rag_mode = RagMode::from_label(&rag_mode_str);

    info!(
        "KB context injection for session {}: rag-mode={}",
        session_id,
        rag_mode.as_str()
    );

    let active_kbs = if mention_config.kbs { get_active_kbs(db_pool, session_id) } else { Vec::new() };
    let active_websites = if mention_config.websites { get_active_websites(db_pool, session_id) } else { Vec::new() };

    if active_kbs.is_empty() && active_websites.is_empty() {
        debug!("No active KBs or websites for session {}", session_id);
        return;
    }

    info!(
        "Injecting context for session {}: {} KB(s), {} website(s)",
        session_id,
        active_kbs.len(),
        active_websites.len()
    );

    let mut all_contexts = Vec::new();

    for kb in &active_kbs {
        let results = super::rag_modes::search_by_mode(
            rag_mode, &kb.qdrant_collection, user_query, 10, bot_id, db_pool,
        ).await;
        if !results.is_empty() {
            let total_tokens: usize = results.iter().map(|r| estimate_tokens(&r.content)).sum();
            info!("Found {} results from KB '{}' using {} mode ({} tokens)", results.len(), kb.kb_name, rag_mode.as_str(), total_tokens);
            all_contexts.push(KbContext {
                kb_name: kb.kb_name.clone(),
                total_tokens,
                search_results: results,
            });
        } else {
            debug!("No results from KB '{}' using {} mode", kb.kb_name, rag_mode.as_str());
        }
    }

    for website in &active_websites {
        let results = super::rag_modes::search_by_mode(
            rag_mode, &website.collection_name, user_query, 10, bot_id, db_pool,
        ).await;
        if !results.is_empty() {
            let total_tokens: usize = results.iter().map(|r| estimate_tokens(&r.content)).sum();
            info!("Found {} results from website '{}' using {} mode ({} tokens)", results.len(), website.website_url, rag_mode.as_str(), total_tokens);
            all_contexts.push(KbContext {
                kb_name: website.website_url.clone(),
                total_tokens,
                search_results: results,
            });
        } else {
            debug!("No results from website '{}' using {} mode", website.website_url, rag_mode.as_str());
        }
    }

    if all_contexts.is_empty() {
        info!("No KB/website content found for session {}", session_id);
        return;
    }

    let context_string = build_context_string(&all_contexts);
    let truncated = truncate_text(&context_string, max_context_tokens);

    if truncated.is_empty() {
        return;
    }

    info!(
        "Injecting {} chars (est. {} tokens) of KB/website context into prompt for session {}",
        truncated.len(),
        estimate_tokens(&truncated),
        session_id
    );

    if let Some(msgs_array) = messages.as_array_mut() {
        if let Some(idx) = msgs_array.iter().position(|m| m["role"] == "system") {
            if let Some(content) = msgs_array[idx]["content"].as_str() {
                msgs_array[idx]["content"] = serde_json::Value::String(format!("{}\n{}", content, truncated));
            }
        } else {
            msgs_array.insert(0, serde_json::json!({
                "role": "system",
                "content": truncated
            }));
        }
    }
}
