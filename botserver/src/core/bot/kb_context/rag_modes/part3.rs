use super::*;

pub(super) async fn graph_search(
    collection: &str,
    query: &str,
    limit: usize,
    bot_id: Uuid,
    db_pool: &DbPool,
) -> Vec<KbSearchResult> {
    info!("Graph RAG search for '{}' in '{}'", query, collection);

    let entities = llm_extract_entities(query, bot_id, db_pool).await;

    if entities.is_empty() {
        debug!("Graph RAG: no entities found, falling back to standard search");
        return kb_context_search::search_qdrant(collection, query, limit, bot_id, db_pool)
            .await
            .unwrap_or_default();
    }

    let mut all_results: Vec<KbSearchResult> = Vec::new();
    for entity in &entities {
        let entity_results = entity_search_term(collection, entity, limit, bot_id, db_pool).await;
        debug!("Graph RAG: entity '{}' returned {} results", entity, entity_results.len());
        all_results.extend(entity_results);
    }

    let original = kb_context_search::search_qdrant(collection, query, limit, bot_id, db_pool)
        .await
        .unwrap_or_default();
    all_results.extend(original);

    all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    all_results.dedup_by(|a, b| a.content == b.content);
    all_results.truncate(limit);

    info!("Graph RAG: {} results after entity merge", all_results.len());
    all_results
}
pub(super) fn decompose_query(query: &str) -> Vec<String> {
    let mut sub_queries = Vec::new();

    for sep in &[" e ", " ou ", ", ", "; ", " vs ", " versus "] {
        if query.contains(sep) {
            let parts: Vec<&str> = query.split(sep).collect();
            if parts.len() >= 2 {
                for part in parts {
                    let trimmed = part.trim();
                    if !trimmed.is_empty() && trimmed.len() > 3 {
                        sub_queries.push(trimmed.to_string());
                    }
                }
                if !sub_queries.is_empty() {
                    sub_queries.push(query.to_string());
                    return sub_queries;
                }
            }
        }
    }

    let words: Vec<&str> = query.split_whitespace().collect();
    if words.len() >= 4 {
        let mid = words.len() / 2;
        sub_queries.push(words[..mid].join(" "));
        sub_queries.push(words[mid..].join(" "));
        sub_queries.push(query.to_string());
    } else {
        sub_queries.push(query.to_string());
    }

    sub_queries.dedup();
    sub_queries
}
pub(super) async fn llm_decompose_query(query: &str, bot_id: Uuid, db_pool: &DbPool) -> Vec<String> {
    let provider = match create_llm_for_bot(bot_id, db_pool).await {
        Some(p) => p,
        None => {
            debug!("Agentic RAG: no LLM available, using heuristic decomposition");
            return decompose_query(query);
        }
    };

    let prompt = format!(
        "Decompose this query into 2-3 focused sub-queries that each target a different aspect of the information needed. Return ONLY the sub-queries, one per line, no numbering.\n\nOriginal query: {}\n\nSub-queries:", query
    );

    match provider.generate_simple(&prompt).await {
        Ok(response) => {
            let sub_queries: Vec<String> = response
                .lines()
                .map(|l| l.trim().trim_matches(|c: char| c == '-' || c == '*' || c.is_ascii_digit()).trim().to_string())
                .filter(|l| !l.is_empty() && l.len() > 5)
                .collect();

            if sub_queries.is_empty() {
                return decompose_query(query);
            }
            let mut result = vec![query.to_string()];
            result.extend(sub_queries);
            result.dedup();
            info!("Agentic RAG: LLM decomposed query into {} sub-queries", result.len());
            result
        }
        Err(_) => decompose_query(query),
    }
}
pub(super) async fn agentic_search(
    collection: &str,
    query: &str,
    limit: usize,
    bot_id: Uuid,
    db_pool: &DbPool,
) -> Vec<KbSearchResult> {
    info!("Agentic RAG search for '{}' in '{}'", query, collection);

    let sub_queries = llm_decompose_query(query, bot_id, db_pool).await;

    let mut all_results: Vec<KbSearchResult> = Vec::new();
    for sub in &sub_queries {
        let results = kb_context_search::search_qdrant(collection, sub, limit, bot_id, db_pool)
            .await
            .unwrap_or_default();
        all_results.extend(results);
    }

    if all_results.is_empty() {
        return kb_context_search::search_keyword_only(collection, query, limit).await;
    }

    all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    all_results.dedup_by(|a, b| a.content == b.content);
    all_results.truncate(limit);

    info!("Agentic RAG: {} results after sub-query merge", all_results.len());
    all_results
}
pub(super) async fn llm_expand_multimodal(query: &str, bot_id: Uuid, db_pool: &DbPool) -> Vec<String> {
    let provider = match create_llm_for_bot(bot_id, db_pool).await {
        Some(p) => p,
        None => {
            debug!("Multimodal RAG: no LLM available, using query as-is");
            return vec![query.to_string()];
        }
    };

    let prompt = format!(
        "A user is searching a multimodal knowledge base that contains documents with images, diagrams, and text. \
        Expand the query to include visual/sensory terms that might describe relevant images, charts, and diagrams. \
        Return the original query and 2 expanded versions, one per line.\n\nOriginal query: {}\n\nExpanded queries:", query
    );

    match provider.generate_simple(&prompt).await {
        Ok(response) => {
            let expanded: Vec<String> = response
                .lines()
                .map(|l| l.trim().trim_matches(|c: char| c == '-' || c == '*' || c.is_ascii_digit()).trim().to_string())
                .filter(|l| !l.is_empty() && l.len() > 5)
                .collect();

            if expanded.is_empty() {
                return vec![query.to_string(), format!("{query} image diagram chart"), format!("{query} visual graph")];
            }
            let mut result = vec![query.to_string()];
            result.extend(expanded);
            result.dedup();
            result.truncate(3);
            info!("Multimodal RAG: expanded query into {} variants", result.len());
            result
        }
        Err(_) => {
            vec![query.to_string(), format!("{query} image diagram chart"), format!("{query} visual graph")]
        }
    }
}
pub(super) async fn multimodal_search(
    collection: &str,
    query: &str,
    limit: usize,
    bot_id: Uuid,
    db_pool: &DbPool,
) -> Vec<KbSearchResult> {
    info!("Multimodal RAG search for '{}' in '{}'", query, collection);

    let variants = llm_expand_multimodal(query, bot_id, db_pool).await;

    let mut all_results: Vec<KbSearchResult> = Vec::new();
    for variant in &variants {
        let results = kb_context_search::search_qdrant(collection, variant, limit, bot_id, db_pool)
            .await
            .unwrap_or_default();
        all_results.extend(results);
    }

    if all_results.is_empty() {
        return kb_context_search::search_keyword_only(collection, query, limit).await;
    }

    all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    all_results.dedup_by(|a, b| a.content == b.content);
    all_results.truncate(limit);

    let has_image_terms: Vec<&str> = vec!["image", "diagram", "chart", "graph", "figure", "table", "visual", "drawing", "photo", "screenshot"];
    for result in &mut all_results {
        let content_lower = result.content.to_lowercase();
        let image_boost = has_image_terms.iter()
            .filter(|&&t| content_lower.contains(t))
            .count() as f32 * 0.1;
        if image_boost > 0.0 {
            result.score = (result.score + image_boost).min(1.0);
        }
    }

    all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    info!("Multimodal RAG: {} results after multi-variant search", all_results.len());
    all_results
}
