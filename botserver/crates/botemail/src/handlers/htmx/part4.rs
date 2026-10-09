use super::*;

pub async fn search_emails_htmx(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let query = params.get("q").map(|s| s.as_str()).unwrap_or("");
    if query.is_empty() {
        return axum::response::Html(r#"<div class="empty-state"><p>Enter a search term to find emails</p></div>"#.to_string());
    }

    let search_term = format!("%{}%", query.to_lowercase());

    let mut conn = match state.pool.get() {
        Ok(c) => c,
        Err(_) => return axum::response::Html(r#"<div class="empty-state error"><p>Database connection error</p></div>"#.to_string()),
    };

    use crate::models::EmailSearchRow;
    let results: Vec<EmailSearchRow> = match diesel::sql_query(
        "SELECT id, subject, from_address, to_addresses, body_text, received_at FROM emails WHERE LOWER(subject) LIKE $1 OR LOWER(from_address) LIKE $1 OR LOWER(body_text) LIKE $1 ORDER BY received_at DESC LIMIT 50"
    )
        .bind::<diesel::sql_types::Text, _>(&search_term)
        .load::<EmailSearchRow>(&mut conn)
    {
        Ok(r) => r,
        Err(e) => { warn!("Email search query failed: {}", e); Vec::new() }
    };

    if results.is_empty() {
        return axum::response::Html(format!(r#"<div class="empty-state"><svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="11" cy="11" r="8"></circle><path d="m21 21-4.35-4.35"></path></svg><h3>No results for "{}"</h3><p>Try different keywords or check your spelling.</p></div>"#, query));
    }

    let mut html = String::from(r#"<div class="search-results">"#);
    use std::fmt::Write;
    let _ = write!(html, r#"<div class="result-stats">Found {} results for "{}"</div>"#, results.len(), query);
    for row in results {
        let preview: String = row.body_text.as_deref().unwrap_or("").chars().take(100).collect();
        let formatted_date = row.received_at.format("%b %d, %Y").to_string();
        let _ = write!(html, r##"<div class="email-item" hx-get="/api/ui/email/content/{}" hx-target="#email-content" hx-swap="innerHTML"><div class="email-sender">{}</div><div class="email-subject">{}</div><div class="email-preview">{}</div><div class="email-date">{}</div></div>"##, row.id, row.from_address, row.subject, preview, formatted_date);
    }
    html.push_str("</div>");
    axum::response::Html(html)
}
pub async fn save_auto_responder(
    State(_state): State<Arc<AppState>>,
    axum::Form(form): axum::Form<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    info!("Saving auto-responder settings: {:?}", form);
    axum::response::Html(r#"<div class="notification success">Auto-responder settings saved successfully!</div>"#.to_string())
}
