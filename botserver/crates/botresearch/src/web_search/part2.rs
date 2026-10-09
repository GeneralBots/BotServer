use super::*;

pub async fn handle_search_history<S: ResearchState>(
    State(state): State<Arc<S>>,
    headers: axum::http::HeaderMap,
    Query(params): Query<SearchHistoryQuery>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).min(100);
    let offset = ((page - 1) * per_page) as i64;

    use diesel::prelude::*;

    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct HistoryRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        pub query: String,
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        pub n: i64,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)]
        pub created_at: chrono::DateTime<Utc>,
    }

    let conn = state.db_pool().clone();
    let branch_id = botsecurity_core::tenant::branch_from_claims(&headers).unwrap_or_else(uuid::Uuid::nil);
    let history: Vec<SearchHistoryEntry> = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {}", e);
                return Vec::new();
            }
        };

        diesel::sql_query(
            "SELECT query, COUNT(*) AS n, MAX(created_at) AS created_at FROM research_searches \
             WHERE query <> '' AND branch_id = $1 GROUP BY query ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind::<diesel::sql_types::Uuid, _>(branch_id)
        .bind::<diesel::sql_types::BigInt, _>(per_page as i64)
        .bind::<diesel::sql_types::BigInt, _>(offset)
        .load::<HistoryRow>(&mut db_conn)
        .map(|rows| {
            rows.into_iter()
                .enumerate()
                .map(|(i, r)| SearchHistoryEntry {
                    id: format!("{i}"),
                    query: r.query,
                    results_count: r.n as usize,
                    timestamp: r.created_at,
                })
                .collect()
        })
        .unwrap_or_default()
    })
    .await
    .unwrap_or_default();

    let mut html = String::new();
    html.push_str("<div class=\"search-history\">");

    if history.is_empty() {
        html.push_str("<div class=\"empty-state\">");
        html.push_str("<p>No search history yet</p>");
        html.push_str("</div>");
    } else {
        for entry in &history {
            html.push_str("<div class=\"history-item\" data-id=\"");
            html.push_str(&html_escape(&entry.id));
            html.push_str("\">");
            html.push_str("<span class=\"history-query\">");
            html.push_str(&html_escape(&entry.query));
            html.push_str("</span>");
            html.push_str("<span class=\"history-count\">");
            html.push_str(&entry.results_count.to_string());
            html.push_str(" results</span>");
            html.push_str("</div>");
        }
    }

    html.push_str("</div>");
    Html(html)
}
pub async fn handle_instant_answer<S: ResearchState>(
    State(_state): State<Arc<S>>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let query = params.get("q").cloned().unwrap_or_default();

    if query.is_empty() {
        return Json(serde_json::json!({
            "answer": null,
            "type": "none"
        }));
    }

    if let Some(answer) = get_instant_answer(&query).await {
        Json(serde_json::json!({
            "answer": answer.0,
            "type": answer.1,
            "source": "duckduckgo"
        }))
    } else {
        Json(serde_json::json!({
            "answer": null,
            "type": "none"
        }))
    }
}
/// Normalizes a search query for scraping-friendly engines: strips accents
/// (DuckDuckGo/Mojeek serve anti-bot challenge pages for %20-encoded or
/// accented queries) and joins words with '+' like a form submission.
pub(super) fn normalize_query(query: &str) -> String {
    let mut out = String::with_capacity(query.len());
    for c in query.chars() {
        let mapped = match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'A',
            'É' | 'È' | 'Ê' | 'Ë' => 'E',
            'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
            'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'O',
            'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
            'Ç' => 'C',
            'Ñ' => 'N',
            _ => c,
        };
        if mapped.is_whitespace() {
            if !out.ends_with('+') {
                out.push('+');
            }
        } else {
            out.push(mapped);
        }
    }
    out.trim_end_matches('+').to_string()
}
pub(super) async fn search_duckduckgo(
    query: &str,
    max_results: usize,
    region: &str,
) -> Result<Vec<WebSearchResult>, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()?;

    let encoded_query = normalize_query(query);
    let url = format!(
        "https://html.duckduckgo.com/html/?q={}&kl={}",
        encoded_query, region
    );

    debug!("Searching DuckDuckGo: {}", query);

    let response = client.get(&url).send().await?;
    let html = response.text().await?;

    let results = parse_duckduckgo_html(&html, max_results);

    info!(
        "DuckDuckGo search for '{}' returned {} results",
        query,
        results.len()
    );

    Ok(results)
}
/// Public wrapper over the DuckDuckGo searcher so the LLM api-command catalog
/// can answer from the web over any channel (e.g. WhatsApp). Falls back to
/// Mojeek when DuckDuckGo returns no results (anti-bot challenge pages).
pub async fn search_web(
    query: &str,
    max_results: usize,
    region: &str,
) -> Result<Vec<WebSearchResult>, String> {
    let ddg = search_duckduckgo(query, max_results, region)
        .await
        .unwrap_or_default();
    if !ddg.is_empty() {
        return Ok(ddg);
    }
    search_mojeek(query, max_results).await
}
/// Mojeek HTML search — a scraping-friendly engine used as a fallback when
/// DuckDuckGo serves its anti-bot challenge page (HTTP 202, zero results).
pub(super) async fn search_mojeek(
    query: &str,
    max_results: usize,
) -> Result<Vec<WebSearchResult>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("https://www.mojeek.com/search?q={}", normalize_query(query));
    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;
    let html = response.text().await.map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    let mut pos = 0;
    while results.len() < max_results {
        let title_start = match html[pos..].find("<h2><a class=\"title\"") {
            Some(i) => pos + i,
            None => break,
        };
        let a_end = match html[title_start..].find("</a></h2>") {
            Some(i) => title_start + i,
            None => break,
        };
        let href_start = match html[title_start..].find("href=\"") {
            Some(i) => title_start + i + 6,
            None => break,
        };
        let href_end = match html[href_start..].find('"') {
            Some(i) => href_start + i,
            None => break,
        };
        let title_start_tag = match html[title_start..].find('>') {
            Some(i) => title_start + i + 1,
            None => break,
        };
        let url = html[href_start..href_end].to_string();
        let title = html[title_start_tag..a_end].trim().to_string();

        let mut snippet = String::new();
        let snip_start = match html[a_end..].find("<p class=\"s\">") {
            Some(i) => a_end + i + 13,
            None => a_end,
        };
        if snip_start != a_end {
            let snip_end = match html[snip_start..].find("</p>") {
                Some(i) => snip_start + i,
                None => snip_start,
            };
            snippet = html[snip_start..snip_end]
                .replace("<b>", "")
                .replace("</b>", "")
                .trim()
                .to_string();
        }

        results.push(WebSearchResult {
            title,
            url,
            snippet,
            source: "mojeek".to_string(),
            favicon: None,
            published_date: None,
        });
        pos = snip_start.max(a_end);
    }

    info!("Mojeek search for '{}' returned {} results", query, results.len());
    Ok(results)
}
