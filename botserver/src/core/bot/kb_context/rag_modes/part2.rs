use super::*;

pub(super) fn reciprocal_rank_fusion(
    vector_results: Vec<KbSearchResult>,
    keyword_results: Vec<KbSearchResult>,
    limit: usize,
) -> Vec<KbSearchResult> {
    let mut rrf_scores: HashMap<String, (f64, KbSearchResult)> = HashMap::new();

    for (rank, result) in vector_results.into_iter().enumerate() {
        let score = 1.0 / (RRF_K + rank as f64);
        rrf_scores.insert(result.content.clone(), (score, result));
    }

    for (rank, result) in keyword_results.into_iter().enumerate() {
        let score = 1.0 / (RRF_K + rank as f64);
        rrf_scores
            .entry(result.content.clone())
            .and_modify(|(existing, _)| *existing += score)
            .or_insert((score, result));
    }

    let mut ranked: Vec<(f64, KbSearchResult)> = rrf_scores.into_values().collect();
    ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    for (_, ref mut result) in &mut ranked {
        result.score = result.score.max(0.0).min(1.0);
    }

    ranked.into_iter().take(limit).map(|(_, r)| r).collect()
}
pub(super) async fn hybrid_search(
    collection: &str,
    query: &str,
    limit: usize,
    bot_id: Uuid,
    db_pool: &DbPool,
) -> Vec<KbSearchResult> {
    info!("Hybrid RAG search for '{}' in '{}'", query, collection);

    let vector_results = kb_context_search::search_qdrant(
        collection, query, limit, bot_id, db_pool,
    )
    .await
    .unwrap_or_default();

    let keyword_results = kb_context_search::search_keyword_only(collection, query, limit).await;

    let vlen = vector_results.len();
    let klen = keyword_results.len();

    if vlen == 0 {
        info!("Hybrid RAG: no vector results, returning keyword results only");
        return keyword_results;
    }
    if klen == 0 {
        info!("Hybrid RAG: no keyword results, returning vector results only");
        return vector_results;
    }

    let fused = reciprocal_rank_fusion(vector_results, keyword_results, limit);
    info!("Hybrid RAG: {} vector + {} keyword -> {} fused", vlen, klen, fused.len());
    fused
}
pub(super) fn expand_query(query: &str) -> Vec<String> {
    let mut queries = vec![query.to_string()];

    let terms: Vec<&str> = query.split_whitespace().collect();
    if terms.len() >= 3 {
        let half = terms.len() / 2;
        queries.push(terms[..half].join(" "));
        queries.push(terms[half..].join(" "));
    }

    queries.dedup();
    queries.retain(|q| !q.is_empty());
    queries
}
pub(super) async fn llm_rewrite_query(query: &str, bot_id: Uuid, db_pool: &DbPool) -> Vec<String> {
    let provider = match create_llm_for_bot(bot_id, db_pool).await {
        Some(p) => p,
        None => {
            debug!("Corrective RAG: no LLM available, using heuristic expansion");
            return expand_query(query);
        }
    };

    let system_prompt = "You are a query expansion assistant. Given a user query, rewrite it into 2-3 search queries that will help retrieve relevant documents from a knowledge base. Return ONLY the queries, one per line, with no numbering or explanation.";
    let prompt = format!("User query: {}\n\nExpanded search queries:", query);

    match provider.generate_simple(&format!("{}\n\n{}", system_prompt, prompt)).await {
        Ok(response) => {
            let rewritten: Vec<String> = response
                .lines()
                .map(|l| l.trim().trim_matches(|c: char| c == '-' || c == '*' || c == '1' || c == '2' || c == '3' || c == '.' || c == ')' || c == ' ').to_string())
                .filter(|l| !l.is_empty() && l.len() > 5)
                .collect();
            if rewritten.is_empty() {
                debug!("Corrective RAG: LLM returned empty result, using heuristic expansion");
                return expand_query(query);
            }
            let mut result = vec![query.to_string()];
            result.extend(rewritten);
            result.dedup();
            info!("Corrective RAG: LLM expanded '{}' into {} queries", query, result.len());
            result
        }
        Err(e) => {
            debug!("Corrective RAG: LLM rewrite failed ({}), using heuristic expansion", e);
            expand_query(query)
        }
    }
}
pub(super) async fn corrective_search(
    collection: &str,
    query: &str,
    limit: usize,
    bot_id: Uuid,
    db_pool: &DbPool,
) -> Vec<KbSearchResult> {
    info!("Corrective RAG search for '{}' in '{}'", query, collection);

    let expanded = llm_rewrite_query(query, bot_id, db_pool).await;

    let mut all_results: Vec<KbSearchResult> = Vec::new();
    for sub_query in &expanded {
        let results = kb_context_search::search_qdrant(
            collection, sub_query, limit, bot_id, db_pool,
        )
        .await
        .unwrap_or_default();
        all_results.extend(results);
    }

    if all_results.is_empty() {
        let keyword = kb_context_search::search_keyword_only(collection, query, limit).await;
        if !keyword.is_empty() {
            info!("Corrective RAG: vector search empty, falling back to keyword");
            return keyword;
        }
        info!("Corrective RAG: local search empty, falling back to web crawl for '{}'", query);
        let config = botcore::kb::web_crawler::WebsiteCrawlConfig {
            url: query.to_string(),
            max_depth: 1,
            max_pages: 3,
            crawl_delay_ms: 200,
            expires_policy: "1h".to_string(),
            refresh_policy: None,
            last_crawled: None,
            next_crawl: None,
        };
        let mut crawler = WebCrawler::new(config);
        match crawler.crawl().await {
            Ok(pages) => {
                    let web_results: Vec<KbSearchResult> = pages.into_iter().map(|p| KbSearchResult {
                        content: p.content,
                        document_path: p.url,
                        score: 0.5,
                    }).collect();
                if !web_results.is_empty() {
                    info!("Corrective RAG: web crawl returned {} results", web_results.len());
                    return web_results;
                }
            }
            Err(e) => debug!("Corrective RAG: web crawl failed: {}", e),
        }
        return Vec::new();
    }

    all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    all_results.dedup_by(|a, b| a.content == b.content);

    let graded = llm_grade_chunks(query, all_results, bot_id, db_pool).await;
    let truncated: Vec<KbSearchResult> = graded.into_iter().take(limit).collect();

    info!("Corrective RAG: {} results after grading", truncated.len());
    truncated
}
pub(super) async fn llm_grade_chunks(
    query: &str,
    results: Vec<KbSearchResult>,
    bot_id: Uuid,
    db_pool: &DbPool,
) -> Vec<KbSearchResult> {
    let provider = match create_llm_for_bot(bot_id, db_pool).await {
        Some(p) => p,
        None => {
            debug!("Corrective RAG: no LLM for grading, returning all results as-is");
            return results;
        }
    };

    if results.len() > 10 {
        debug!("Corrective RAG: grading {} chunks individually via LLM", results.len());
    }

    let mut graded = Vec::new();
    for chunk in results {
        let grade_prompt = format!(
            "Query: {}\n\nDocument chunk:\n{}\n\nRelevance (0-10):",
            query, chunk.content
        );
        let system_grade = "You are a relevance grader. Rate how relevant this document chunk is to the query on a scale of 0 (completely irrelevant) to 10 (perfectly relevant). Return ONLY a single number.";

        match provider.generate_simple(&format!("{}\n\n{}", system_grade, grade_prompt)).await {
            Ok(response) => {
                let grade: f32 = response.trim().chars()
                    .filter(|c| c.is_ascii_digit() || *c == '.')
                    .collect::<String>()
                    .parse()
                    .unwrap_or(5.0);
                let boost = grade / 10.0;
                let mut graded_chunk = chunk;
                graded_chunk.score = (graded_chunk.score + boost) / 2.0;
                if grade >= 3.0 {
                    graded.push(graded_chunk);
                }
            }
            Err(_) => {
                graded.push(chunk);
            }
        }
    }

    graded.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    graded
}
pub(super) fn extract_entities(query: &str) -> Vec<String> {
    let mut entities = Vec::new();

    for word in query.split_whitespace() {
        let clean = word.trim_matches(|c: char| c.is_ascii_punctuation());
        if clean.len() >= 3 && clean.chars().next().is_some_and(|c| c.is_uppercase()) {
            entities.push(clean.to_string());
        }
    }

    let multi_word: Vec<&str> = query.split([',', ';', '!'])
        .map(|s| s.trim())
        .filter(|s| s.chars().filter(|&c| c == ' ').count() >= 1 && s.len() > 5)
        .collect();

    for phrase in multi_word {
        let has_upper = phrase.chars().any(|c| c.is_uppercase());
        if has_upper && !entities.iter().any(|e| phrase.contains(e.as_str())) {
            entities.push(phrase.to_string());
        }
    }

    entities.dedup();
    entities.truncate(5);
    entities
}
pub(super) async fn llm_extract_entities(query: &str, bot_id: Uuid, db_pool: &DbPool) -> Vec<String> {
    let provider = match create_llm_for_bot(bot_id, db_pool).await {
        Some(p) => p,
        None => {
            debug!("Graph RAG: no LLM for entity extraction, using heuristic");
            return extract_entities(query);
        }
    };

    let prompt = format!(
        "Extract named entities (people, organizations, places, products, concepts) from this query. Return ONLY the entity names, one per line, no numbering.\n\nQuery: {}\n\nEntities:", query
    );

    match provider.generate_simple(&prompt).await {
        Ok(response) => {
            let entities: Vec<String> = response
                .lines()
                .map(|l| l.trim().trim_matches(|c: char| c == '-' || c == '*' || c.is_ascii_digit()).trim().to_string())
                .filter(|l| !l.is_empty() && l.len() > 2)
                .collect();

            if entities.is_empty() {
                return extract_entities(query);
            }
            debug!("Graph RAG: LLM extracted {} entities: {:?}", entities.len(), entities);
            entities.into_iter().take(5).collect()
        }
        Err(_) => extract_entities(query),
    }
}
pub(super) async fn entity_search_term(
    collection: &str,
    term: &str,
    limit: usize,
    bot_id: Uuid,
    db_pool: &DbPool,
) -> Vec<KbSearchResult> {
    let results = kb_context_search::search_qdrant(collection, term, limit, bot_id, db_pool)
        .await
        .unwrap_or_default();

    if !results.is_empty() {
        return results;
    }

    kb_context_search::search_keyword_only(collection, term, limit).await
}
