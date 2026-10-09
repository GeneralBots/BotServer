use super::*;

pub async fn handle_search<S: ResearchState>(
    State(state): State<Arc<S>>,
    headers: axum::http::HeaderMap,
    Form(payload): Form<SearchRequest>,
) -> impl IntoResponse {
    let query = payload.query.unwrap_or_default();

    if query.trim().is_empty() {
        return Html(
            "<div class=\"search-prompt\"><p>Enter a search query to find relevant documents</p></div>"
                .to_string(),
        );
    }

    let conn = state.db_pool().clone();
    let collection = payload.collection;
    let bot_id = state.bot_id();
    let scope = bot_scope_clause(&bot_id);
    let query_c = query.clone();
    let branch_id = botsecurity_core::tenant::branch_from_claims(&headers).unwrap_or_else(uuid::Uuid::nil);

    let results = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {e}");
                return Vec::new();
            }
        };

        // Record the real search so Recent/Trending reflect actual activity.
        // Scoped to the caller's branch (issue #734): one tenant's search
        // history is never visible to another tenant.
        let _ = diesel::sql_query(
            "INSERT INTO research_searches (query, branch_id) VALUES ($1, $2)",
        )
        .bind::<diesel::sql_types::Text, _>(&query_c)
        .bind::<diesel::sql_types::Uuid, _>(branch_id)
        .execute(&mut db_conn);

        let search_pattern = format!("%{}%", query_c.to_lowercase());

        if let Some(coll) = collection {
            diesel::sql_query(&format!(
                "SELECT id, file_path, collection_name, metadata FROM kb_documents WHERE (LOWER(file_path) LIKE $1 OR metadata::text ILIKE $1) AND collection_name = $2{} ORDER BY file_path ASC LIMIT 20",
                scope
            ))
            .bind::<diesel::sql_types::Text, _>(&search_pattern)
            .bind::<diesel::sql_types::Text, _>(&coll)
            .load::<KbDocumentRow>(&mut db_conn)
            .unwrap_or_default()
        } else {
            diesel::sql_query(&format!(
                "SELECT id, file_path, collection_name, metadata FROM kb_documents WHERE LOWER(file_path) LIKE $1 OR metadata::text ILIKE $1{} ORDER BY file_path ASC LIMIT 20",
                scope
            ))
            .bind::<diesel::sql_types::Text, _>(&search_pattern)
            .load::<KbDocumentRow>(&mut db_conn)
            .unwrap_or_default()
        }
    })
    .await
    .unwrap_or_default();

    let llm = state.llm_provider();

    // When the LLM is available, synthesize a real RAG answer with citations.
    if let Some(llm) = llm {
        let context = results
            .iter()
            .map(|d| format!("- {} ({})", d.file_path, d.collection_name))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Answer the user's research question using ONLY the knowledge base excerpts provided.\n\
             Question: {query}\n\n\
             Knowledge base excerpts:\n{context}\n\n\
             Provide a concise, factual answer with inline citations referencing the source file names."
        );

        let answer = match llm.generate_simple(&prompt).await {
            Ok(a) => a,
            Err(e) => {
                log::error!("Research LLM call failed: {e}");
                String::new()
            }
        };

        if !answer.trim().is_empty() {
            let mut html = String::new();
            html.push_str("<div class=\"search-results\">");
            html.push_str("<div class=\"results-header\">");
            html.push_str("<h3>AI Answer</h3>");
            html.push_str("<span class=\"result-count\">");
            html.push_str(&results.len().to_string());
            html.push_str(" sources</span>");
            html.push_str("</div>");
            html.push_str("<div class=\"ai-answer\" id=\"answer-content\">");
            html.push_str(&html_escape(&answer).replace('\n', "<br>"));
            html.push_str("</div>");
            html.push_str("<div class=\"results-list\">");
            for doc in &results {
                html.push_str(&format_search_result(doc));
            }
            html.push_str("</div></div>");
            return Html(html);
        }
    }

    let mut html = String::new();
    html.push_str("<div class=\"search-results\">");
    html.push_str("<div class=\"results-header\">");
    html.push_str("<h3>Search Results</h3>");
    html.push_str("<span class=\"result-count\">");
    html.push_str(&results.len().to_string());
    html.push_str(" results found</span>");
    html.push_str("</div>");
    html.push_str("<div class=\"results-list\">");

    if results.is_empty() {
        html.push_str("<div class=\"no-results\">");
        html.push_str("<div class=\"no-results-icon\"></div>");
        html.push_str("<h4>No results found</h4>");
        html.push_str("<p>Try different keywords or check your spelling</p>");
        html.push_str("</div>");
    } else {
        for doc in &results {
            html.push_str(&format_search_result(doc));
        }
    }

    html.push_str("</div>");
    html.push_str("</div>");

    Html(html)
}
pub(super) fn format_search_result(doc: &KbDocumentRow) -> String {
    let title = doc.file_path.rsplit('/').next().unwrap_or(&doc.file_path);
    let snippet = doc
        .metadata
        .as_ref()
        .and_then(|m| m.get("summary").and_then(|s| s.as_str()))
        .unwrap_or("Knowledge base document")
        .to_string();

    let mut html = String::new();
    html.push_str("<div class=\"result-item\" data-id=\"");
    html.push_str(&html_escape(&doc.id));
    html.push_str("\">");
    html.push_str("<div class=\"result-header\">");
    html.push_str("<h4 class=\"result-title\">");
    html.push_str(&html_escape(title));
    html.push_str("</h4>");
    html.push_str("<span class=\"result-source\">");
    html.push_str(&html_escape(&doc.collection_name));
    html.push_str("</span>");
    html.push_str("</div>");
    html.push_str("<p class=\"result-snippet\">");
    html.push_str(&html_escape(&snippet));
    html.push_str("</p>");
    html.push_str("<div class=\"result-actions\">");
    html.push_str("<button class=\"btn-sm btn-view\">View</button>");
    html.push_str("<button class=\"btn-sm btn-cite\">Cite</button>");
    html.push_str("<button class=\"btn-sm btn-save\">Save</button>");
    html.push_str("</div>");
    html.push_str("</div>");

    html
}
pub async fn handle_recent_searches<S: ResearchState>(
    State(state): State<Arc<S>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let conn = state.db_pool().clone();
    let branch_id = botsecurity_core::tenant::branch_from_claims(&headers).unwrap_or_else(uuid::Uuid::nil);

    let recent_searches: Vec<String> = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {}", e);
                return Vec::new();
            }
        };

        diesel::sql_query(
            "SELECT query FROM research_searches WHERE query <> '' AND branch_id = $1 \
             ORDER BY created_at DESC LIMIT 8",
        )
        .bind::<diesel::sql_types::Uuid, _>(branch_id)
        .load::<RecentSearchRow>(&mut db_conn)
        .map(|rows| rows.into_iter().map(|r| r.query).collect())
        .unwrap_or_default()
    })
    .await
    .unwrap_or_default();

    let mut html = String::new();

    for search in &recent_searches {
        html.push_str(
            "<div class=\"recent-item\" hx-post=\"/api/ui/research/search\" hx-vals='{\"query\":\"",
        );
        html.push_str(&html_escape(search));
        html.push_str("\"}' hx-target=\"#main-results\">");
        html.push_str("<span class=\"recent-icon\">\u{1F550}</span>");
        html.push_str("<span class=\"recent-text\">");
        html.push_str(&html_escape(search));
        html.push_str("</span>");
        html.push_str("</div>");
    }

    if recent_searches.is_empty() {
        html.push_str("<div class=\"empty-state small\">");
        html.push_str("<p>No recent searches yet</p>");
        html.push_str("</div>");
    }

    Html(html)
}
pub async fn handle_trending_tags<S: ResearchState>(
    State(state): State<Arc<S>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let conn = state.db_pool().clone();
    let branch_id = botsecurity_core::tenant::branch_from_claims(&headers).unwrap_or_else(uuid::Uuid::nil);

    let tags: Vec<(String, i64)> = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {}", e);
                return Vec::new();
            }
        };

        diesel::sql_query(
            "SELECT query, COUNT(*) AS n FROM research_searches \
             WHERE query <> '' AND branch_id = $1 GROUP BY query \
             ORDER BY n DESC, MAX(created_at) DESC LIMIT 8",
        )
        .bind::<diesel::sql_types::Uuid, _>(branch_id)
        .load::<TrendingTagRow>(&mut db_conn)
        .map(|rows| rows.into_iter().map(|r| (r.query, r.n)).collect())
        .unwrap_or_default()
    })
    .await
    .unwrap_or_default();

    let mut html = String::new();
    html.push_str("<div class=\"trending-tags-list\">");

    for (tag, count) in &tags {
        html.push_str(
            "<span class=\"tag\" hx-post=\"/api/ui/research/search\" hx-vals='{\"query\":\"",
        );
        html.push_str(&html_escape(tag));
        html.push_str("\"}' hx-target=\"#main-results\">");
        html.push_str(&html_escape(tag));
        html.push_str(" <small>(");
        html.push_str(&count.to_string());
        html.push_str(")</small>");
        html.push_str("</span>");
    }

    html.push_str("</div>");

    Html(html)
}
