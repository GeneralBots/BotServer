use super::*;

pub fn configure_web_search_routes<S: ResearchState>() -> Router<Arc<S>> {
    Router::new()
        .route(ROUTE_RESEARCH_WEB_SEARCH, post(handle_web_search::<S>))
        .route(
            ROUTE_RESEARCH_WEB_SUMMARIZE,
            post(handle_summarize::<S>),
        )
        .route(ROUTE_RESEARCH_WEB_DEEP, post(handle_deep_research::<S>))
        .route(ROUTE_RESEARCH_WEB_HISTORY, get(handle_search_history::<S>))
        .route(ROUTE_RESEARCH_WEB_INSTANT, get(handle_instant_answer::<S>))
}
pub async fn handle_web_search<S: ResearchState>(
    State(_state): State<Arc<S>>,
    Json(payload): Json<WebSearchRequest>,
) -> impl IntoResponse {
    let start_time = std::time::Instant::now();

    if payload.query.trim().is_empty() {
        return Json(WebSearchResponse {
            results: Vec::new(),
            query: payload.query,
            total_results: 0,
            search_time_ms: 0,
            source: "none".to_string(),
        });
    }

    let max_results = payload.max_results.unwrap_or(10).min(25);
    let region = payload.region.as_deref().unwrap_or("wt-wt");

    let results = match search_duckduckgo(&payload.query, max_results, region).await {
        Ok(r) => r,
        Err(e) => {
            log::error!("DuckDuckGo search failed: {}", e);
            Vec::new()
        }
    };

    let search_time_ms = start_time.elapsed().as_millis() as u64;

    Json(WebSearchResponse {
        total_results: results.len(),
        results,
        query: payload.query,
        search_time_ms,
        source: "duckduckgo".to_string(),
    })
}
pub async fn handle_summarize<S: ResearchState>(
    State(state): State<Arc<S>>,
    Json(payload): Json<SummarizeRequest>,
) -> impl IntoResponse {
    if payload.results.is_empty() {
        return Json(SummarizeResponse {
            summary: "No results to summarize.".to_string(),
            citations: Vec::new(),
            confidence: 0.0,
        });
    }

    let mut combined_text = String::new();
    let mut citations = Vec::new();

    for (idx, result) in payload.results.iter().enumerate() {
        let _ = writeln!(combined_text, "[{}] {}\n{}", idx + 1, result.title, result.snippet);
        citations.push(Citation {
            index: idx + 1,
            title: result.title.clone(),
            url: result.url.clone(),
            relevance: 1.0 - (idx as f32 * 0.1).min(0.5),
        });
    }

    let max_len = payload.max_length.unwrap_or(500);

    // Use the real LLM when available to synthesize the summary from the snippets.
    let summary = if let Some(llm) = state.llm_provider() {
        let prompt = format!(
            "Summarize the following search results about the query. Be concise and factual, \
             cite sources by their bracketed number [N] inline.\n\n{combined_text}"
        );
        match llm.generate_simple(&prompt).await {
            Ok(answer) => {
                let mut truncated = answer.trim().to_string();
                if truncated.chars().count() > max_len * 2 {
                    truncated = truncated.chars().take(max_len * 2).collect::<String>();
                    truncated.push_str("...");
                }
                truncated
            }
            Err(e) => {
                log::error!("Research summarize LLM call failed: {e}");
                let mut truncated = combined_text.chars().take(max_len).collect::<String>();
                if let Some(last_period) = truncated.rfind(". ") {
                    truncated.truncate(last_period + 1);
                }
                truncated
            }
        }
    } else {
        let mut truncated = combined_text.chars().take(max_len).collect::<String>();
        if let Some(last_period) = truncated.rfind(". ") {
            truncated.truncate(last_period + 1);
        }
        truncated
    };

    let confidence = (payload.results.len() as f32 / 10.0).min(1.0);

    Json(SummarizeResponse {
        summary,
        citations,
        confidence,
    })
}
pub async fn handle_deep_research<S: ResearchState>(
    State(state): State<Arc<S>>,
    Json(payload): Json<DeepResearchRequest>,
) -> impl IntoResponse {
    let start_time = std::time::Instant::now();

    if payload.query.trim().is_empty() {
        return Json(DeepResearchResponse {
            answer: "Please provide a research query.".to_string(),
            sources: Vec::new(),
            citations: Vec::new(),
            related_queries: Vec::new(),
            confidence: 0.0,
            research_time_ms: 0,
        });
    }

    let depth = payload.depth.unwrap_or(2).min(3);
    let max_sources = payload.max_sources.unwrap_or(10).min(20);

    let mut all_results: Vec<WebSearchResult> = Vec::new();
    let mut seen_urls: std::collections::HashSet<String> = std::collections::HashSet::new();

    let initial_results = search_duckduckgo(&payload.query, max_sources, "wt-wt")
        .await
        .unwrap_or_default();

    for result in initial_results {
        if !seen_urls.contains(&result.url) {
            seen_urls.insert(result.url.clone());
            all_results.push(result);
        }
    }

    if depth > 1 {
        let related_queries = generate_related_queries(&payload.query);

        for rq in related_queries.iter().take(depth - 1) {
            if let Ok(more_results) = search_duckduckgo(rq, 5, "wt-wt").await {
                for result in more_results {
                    if !seen_urls.contains(&result.url) && all_results.len() < max_sources {
                        seen_urls.insert(result.url.clone());
                        all_results.push(result);
                    }
                }
            }
        }
    }

    let mut citations = Vec::new();
    let mut answer_parts: Vec<String> = Vec::new();

    for (idx, result) in all_results.iter().enumerate() {
        if idx < 5 {
            answer_parts.push(format!("\u{2022} {}", result.snippet));
        }
        citations.push(Citation {
            index: idx + 1,
            title: result.title.clone(),
            url: result.url.clone(),
            relevance: 1.0 - (idx as f32 * 0.05).min(0.5),
        });
    }

    // Use the real LLM when available to synthesize a deep research answer.
    let answer = if let Some(llm) = state.llm_provider() {
        if answer_parts.is_empty() {
            format!("No results found for: {}", payload.query)
        } else {
            let sources_block = all_results
                .iter()
                .enumerate()
                .map(|(i, r)| format!("[{}] {} — {}", i + 1, r.title, r.snippet))
                .collect::<Vec<_>>()
                .join("\n");
            let prompt = format!(
                "Perform deep research on the question. Synthesize a structured, factual answer \
                 using ONLY the sources below, citing them inline as [N].\n\n\
                 Question: {}\n\nSources:\n{}",
                payload.query, sources_block
            );
            match llm.generate_simple(&prompt).await {
                Ok(a) => a,
                Err(e) => {
                    log::error!("Research deep LLM call failed: {e}");
                    if answer_parts.is_empty() {
                        format!("No results found for: {}", payload.query)
                    } else {
                        format!(
                            "Based on {} sources about \"{}\":\n\n{}",
                            all_results.len(),
                            payload.query,
                            answer_parts.join("\n\n")
                        )
                    }
                }
            }
        }
    } else if answer_parts.is_empty() {
        format!("No results found for: {}", payload.query)
    } else {
        format!(
            "Based on {} sources about \"{}\":\n\n{}",
            all_results.len(),
            payload.query,
            answer_parts.join("\n\n")
        )
    };

    let related = generate_related_queries(&payload.query);

    let research_time_ms = start_time.elapsed().as_millis() as u64;
    let confidence = (citations.len() as f32 / 10.0).min(1.0);

    Json(DeepResearchResponse {
        answer,
        sources: all_results,
        citations,
        related_queries: related,
        confidence,
        research_time_ms,
    })
}
