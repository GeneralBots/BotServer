use super::*;

pub async fn handle_prompts<S: ResearchState>(
    State(state): State<Arc<S>>,
) -> impl IntoResponse {
    let bot_id = state.bot_id();
    let scope = bot_scope_clause(&bot_id);

    let conn = state.db_pool().clone();
    let topics: Vec<String> = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {}", e);
                return Vec::new();
            }
        };

        diesel::sql_query(&format!(
            "SELECT name FROM kb_collections WHERE 1 = 1{} ORDER BY document_count DESC LIMIT 6",
            scope
        ))
        .load::<CollectionNameRow>(&mut db_conn)
        .map(|rows| rows.into_iter().map(|r| r.name).collect())
        .unwrap_or_default()
    })
    .await
    .unwrap_or_default();

    let mut prompts: Vec<(String, String, String)> = topics
        .iter()
        .map(|t| ("".to_string(), t.clone(), format!("Explore the {t} knowledge base")))
        .collect();

    if prompts.is_empty() {
        prompts = vec![
            ("\u{1F50C}".to_string(), "Summarize my knowledge base".to_string(), "Get an overview of your uploaded documents".to_string()),
            ("\u{1F4C4}".to_string(), "Find recent documents".to_string(), "List the most recently indexed files".to_string()),
            ("\u{1F50D}".to_string(), "Search across sources".to_string(), "Query documents by topic or file name".to_string()),
        ];
    }

    let mut html = String::new();
    html.push_str("<div class=\"prompts-grid\">");

    for (icon, title, description) in &prompts {
        html.push_str(
            "<div class=\"prompt-card\" hx-post=\"/api/ui/research/search\" hx-vals='{\"query\":\"",
        );
        html.push_str(&html_escape(title));
        html.push_str("\"}' hx-target=\"#main-results\">");
        html.push_str("<div class=\"prompt-icon\">");
        html.push_str(icon);
        html.push_str("</div>");
        html.push_str("<div class=\"prompt-content\">");
        html.push_str("<h4>");
        html.push_str(&html_escape(title));
        html.push_str("</h4>");
        html.push_str("<p>");
        html.push_str(&html_escape(description));
        html.push_str("</p>");
        html.push_str("</div>");
        html.push_str("</div>");
    }

    html.push_str("</div>");

    Html(html)
}
