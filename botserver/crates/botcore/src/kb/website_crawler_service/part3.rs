use super::*;

pub(super) fn sanitize_url_for_kb(url: &str) -> String {
    url.replace("http://", "")
        .replace("https://", "")
        .replace(['/', ':', '.'], "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect::<String>()
        .to_lowercase()
}
/// Force recrawl a specific website immediately
/// Updates next_crawl to NOW() and triggers immediate crawl
pub async fn force_recrawl_website(
    db_pool: crate::shared::utils::DbPool,
    bot_id: uuid::Uuid,
    url: String,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    use diesel::prelude::*;

    let mut conn = db_pool.get()?;

    // Update next_crawl to NOW() to mark for immediate recrawl
    let rows_updated = diesel::sql_query(
        "UPDATE website_crawls
         SET next_crawl = NOW(),
             crawl_status = 0,
             error_message = NULL
         WHERE bot_id = $1 AND url = $2"
    )
    .bind::<diesel::sql_types::Uuid, _>(&bot_id)
    .bind::<diesel::sql_types::Text, _>(&url)
    .execute(&mut conn)?;

    if rows_updated == 0 {
        return Err(format!("Website not found: bot_id={}, url={}", bot_id, url).into());
    }

    trace!("Updated next_crawl to NOW() for website: {}", url);

    // Get the website record for immediate crawling
    #[derive(diesel::QueryableByName)]
    struct WebsiteRecord {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: uuid::Uuid,
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        bot_id: uuid::Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)]
        url: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        expires_policy: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        refresh_policy: String,
        #[diesel(sql_type = diesel::sql_types::Integer)]
        max_depth: i32,
        #[diesel(sql_type = diesel::sql_types::Integer)]
        max_pages: i32,
    }

    let website: WebsiteRecord = diesel::sql_query(
        "SELECT id, bot_id, url, expires_policy, refresh_policy, max_depth, max_pages
         FROM website_crawls
         WHERE bot_id = $1 AND url = $2"
    )
    .bind::<diesel::sql_types::Uuid, _>(&bot_id)
    .bind::<diesel::sql_types::Text, _>(&url)
    .get_result(&mut conn)?;

    // Convert to WebsiteCrawlRecord
    let website_record = WebsiteCrawlRecord {
        id: website.id,
        bot_id: website.bot_id,
        url: website.url.clone(),
        expires_policy: website.expires_policy,
        refresh_policy: Some(website.refresh_policy),
        max_depth: website.max_depth,
        max_pages: website.max_pages,
        next_crawl: None,
        crawl_status: Some(0),
    };

    // Trigger immediate crawl
    let active_crawls = Arc::new(tokio::sync::RwLock::new(HashSet::new()));

    trace!("Starting immediate crawl for website: {}", url);

    match WebsiteCrawlerService::crawl_website(website_record, db_pool, active_crawls).await {
        Ok(_) => {
            let msg = format!("Successfully triggered immediate recrawl for website: {}", url);
            info!("{}", msg);
            Ok(msg)
        }
        Err(e) => {
            let error_msg = format!("Failed to crawl website {}: {}", url, e);
            log::error!("{}", error_msg);
            Err(error_msg.into())
        }
    }
}
/// API Handler for force recrawl endpoint
/// POST /api/website/force-recrawl
/// Body: {"bot_id": "uuid", "url": "https://example.com"}
pub async fn handle_force_recrawl(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<AppState>>,
    axum::Json(payload): axum::Json<ForceRecrawlRequest>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    // TODO: log_and_sanitize_str - use inline error handling
// use botsecurity_core::error_sanitizer::log_and_sanitize_str;

    match force_recrawl_website(
        state.conn.clone(),
        payload.bot_id,
        payload.url.clone(),
    ).await {
        Ok(msg) => Ok(axum::Json(serde_json::json!({
            "success": true,
            "message": msg,
            "bot_id": payload.bot_id,
            "url": payload.url
        }))),
        Err(e) => {
            log::error!("force_recrawl error: {}", e);
            let sanitized = "An internal error occurred".to_string();
            Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({"error": sanitized}))
            ))
        }
    }
}
