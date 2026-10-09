use super::*;

pub(super) fn check_website_crawl_status(
    conn: &mut PgConnection,
    bot_id: &Uuid,
    url: &str,
) -> Result<WebsiteCrawlStatus, String> {
    #[derive(QueryableByName)]
    struct CrawlStatus {
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::SmallInt>)]
        crawl_status: Option<i16>,
    }

    let query =
        diesel::sql_query("SELECT crawl_status FROM website_crawls WHERE bot_id = $1 AND url = $2")
            .bind::<diesel::sql_types::Uuid, _>(bot_id)
            .bind::<diesel::sql_types::Text, _>(url);

    let result: Result<CrawlStatus, _> = query.get_result(conn);

    match result {
        Ok(status) => match status.crawl_status {
            Some(0) => Ok(WebsiteCrawlStatus::Pending),
            Some(1) => Ok(WebsiteCrawlStatus::Crawled),
            Some(2) => Ok(WebsiteCrawlStatus::Failed),
            _ => Ok(WebsiteCrawlStatus::NotRegistered),
        },
        Err(_) => Ok(WebsiteCrawlStatus::NotRegistered),
    }
}
pub fn register_website_for_crawling(
    conn: &mut PgConnection,
    bot_id: &Uuid,
    url: &str,
) -> Result<(), String> {
    register_website_for_crawling_with_refresh(conn, bot_id, url, "1m") // Default: 1 month
}
pub fn register_website_for_crawling_with_refresh(
    conn: &mut PgConnection,
    bot_id: &Uuid,
    url: &str,
    refresh_interval: &str,
) -> Result<(), String> {
    // Check if already registered to avoid misleading logs
    let is_new: bool = {
        #[derive(QueryableByName)]
        struct ExistsRow {
            #[diesel(sql_type = diesel::sql_types::BigInt)]
            cnt: i64,
        }
        let result: Vec<ExistsRow> = diesel::sql_query(
            "SELECT COUNT(*)::BIGINT as cnt FROM website_crawls WHERE bot_id = $1 AND url = $2",
        )
        .bind::<diesel::sql_types::Uuid, _>(bot_id)
        .bind::<diesel::sql_types::Text, _>(url)
        .load(conn)
        .unwrap_or_default();

        result.first().map(|r| r.cnt == 0).unwrap_or(true)
    };

    let days = parse_refresh_interval(refresh_interval)
        .map_err(|e| format!("Invalid refresh interval: {}", e))?;

    let expires_policy = days_to_expires_policy(days);

    let query = diesel::sql_query(
        "WITH upsert AS (
            UPDATE website_crawls SET
                next_crawl = CASE WHEN crawl_status = 2 THEN NOW() ELSE next_crawl END,
                refresh_policy = CASE
                    WHEN website_crawls.refresh_policy IS NULL THEN $4
                    ELSE LEAST(website_crawls.refresh_policy, $4)
                END
            WHERE bot_id = $1 AND url = $2
            RETURNING id
        )
        INSERT INTO website_crawls (id, bot_id, url, expires_policy, crawl_status, next_crawl, refresh_policy)
        SELECT gen_random_uuid(), $1, $2, $3, 0, NOW(), $4
        WHERE NOT EXISTS (SELECT 1 FROM upsert)",
    )
    .bind::<diesel::sql_types::Uuid, _>(bot_id)
    .bind::<diesel::sql_types::Text, _>(url)
    .bind::<diesel::sql_types::Text, _>(expires_policy)
    .bind::<diesel::sql_types::Text, _>(refresh_interval);

    query
        .execute(conn)
        .map_err(|e| format!("Failed to register website for crawling: {}", e))?;

    if is_new {
        info!(
            "Website {} registered for crawling for bot {} with refresh policy: {}",
            url, bot_id, refresh_interval
        );
    } else {
        debug!(
            "Website {} already registered for crawling for bot {}, refresh policy: {}",
            url, bot_id, refresh_interval
        );
    }
    Ok(())
}
/// Update refresh policy if the new interval is shorter than the existing one.
/// When the policy is updated, triggers an immediate crawl by setting next_crawl=NOW()
/// and crawl_status=0, ensuring the website is recrawled on the next crawler cycle
/// regardless of the previous schedule.
pub(super) fn update_refresh_policy_if_shorter(
    conn: &mut PgConnection,
    bot_id: &Uuid,
    url: &str,
    refresh_interval: &str,
) -> Result<(), String> {
    // Get current record to compare in Rust (no SQL business logic!)
    #[derive(QueryableByName)]
    struct CurrentRefresh {
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        refresh_policy: Option<String>,
    }

    let current = diesel::sql_query(
        "SELECT refresh_policy FROM website_crawls WHERE bot_id = $1 AND url = $2",
    )
    .bind::<diesel::sql_types::Uuid, _>(bot_id)
    .bind::<diesel::sql_types::Text, _>(url)
    .get_result::<CurrentRefresh>(conn)
    .ok();

    let new_days = parse_refresh_interval(refresh_interval)
        .map_err(|e| format!("Invalid refresh interval: {}", e))?;

    // Check if we should update (no policy exists or new interval is shorter)
    let should_update = match &current {
        Some(c) if c.refresh_policy.is_some() => {
            let existing_days = if let Some(ref policy) = c.refresh_policy {
                parse_refresh_interval(policy).unwrap_or(i32::MAX)
            } else {
                i32::MAX
            };
            new_days < existing_days
        }
        _ => true, // No existing policy, so update
    };

    if should_update {
        let expires_policy = days_to_expires_policy(new_days);

        diesel::sql_query(
            "UPDATE website_crawls SET refresh_policy = $3, expires_policy = $4, next_crawl = NOW(), crawl_status = 0
             WHERE bot_id = $1 AND url = $2"
        )
        .bind::<diesel::sql_types::Uuid, _>(bot_id)
        .bind::<diesel::sql_types::Text, _>(url)
        .bind::<diesel::sql_types::Text, _>(refresh_interval)
        .bind::<diesel::sql_types::Text, _>(expires_policy)
        .execute(conn)
        .map_err(|e| format!("Failed to update refresh policy: {}", e))?;

        info!(
            "Refresh policy updated to {} for {} - immediate crawl scheduled",
            refresh_interval, url
        );
    }

    Ok(())
}
pub fn execute_use_website_preprocessing(
    conn: &mut PgConnection,
    url: &str,
    bot_id: Uuid,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    execute_use_website_preprocessing_with_refresh(conn, url, bot_id, "1m") // Default: 1 month
}
pub fn execute_use_website_preprocessing_with_refresh(
    conn: &mut PgConnection,
    url: &str,
    bot_id: Uuid,
    refresh_interval: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    trace!(
        "Preprocessing USE_WEBSITE: {}, bot_id: {:?}, refresh: {}",
        url,
        bot_id,
        refresh_interval
    );

    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(format!(
            "Invalid URL format: {}. Must start with http:// or https://",
            url
        )
        .into());
    }

    register_website_for_crawling_with_refresh(conn, &bot_id, url, refresh_interval)?;

    Ok(serde_json::json!({
        "command": "use_website",
        "url": url,
        "bot_id": bot_id.to_string(),
        "refresh_policy": refresh_interval,
        "status": "registered_for_crawling"
    }))
}
pub(super) fn add_website_to_session(
    conn: &mut PgConnection,
    session_id: &Uuid,
    bot_id: &Uuid,
    url: &str,
    collection_name: &str,
) -> Result<(), String> {
    let assoc_id = Uuid::new_v4();

    diesel::sql_query(
        "INSERT INTO session_website_associations
         (id, session_id, bot_id, website_url, collection_name, is_active, added_at)
         VALUES ($1, $2, $3, $4, $5, true, NOW())
         ON CONFLICT (session_id, website_url)
         DO UPDATE SET is_active = true, added_at = NOW()",
    )
    .bind::<diesel::sql_types::Uuid, _>(assoc_id)
    .bind::<diesel::sql_types::Uuid, _>(session_id)
    .bind::<diesel::sql_types::Uuid, _>(bot_id)
    .bind::<diesel::sql_types::Text, _>(url)
    .bind::<diesel::sql_types::Text, _>(collection_name)
    .execute(conn)
    .map_err(|e| format!("Failed to add website to session: {}", e))?;

    info!(
        " Added website '{}' to session {} (collection: {})",
        url, session_id, collection_name
    );

    Ok(())
}
pub fn clear_websites_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine
        .register_custom_syntax(["CLEAR", "WEBSITES"], true, move |_context, _inputs| {
            info!(
                "CLEAR WEBSITES keyword executed for session: {}",
                user_clone.id
            );

            let session_id = user_clone.id;
            let conn = state_clone.db_pool().clone();

            let result = std::thread::spawn(move || clear_all_websites(conn, session_id)).join();

            match result {
                Ok(Ok(count)) => {
                    info!(
                        "Successfully cleared {} websites from session {}",
                        count, user_clone.id
                    );
                    Ok(Dynamic::from(format!(
                        "{} website(s) removed from conversation",
                        count
                    )))
                }
                Ok(Err(e)) => {
                    log::error!("Failed to clear websites: {}", e);
                    Err(format!("CLEAR_WEBSITES failed: {}", e).into())
                }
                Err(e) => {
                    log::error!("Thread panic in CLEAR_WEBSITES: {:?}", e);
                    Err("CLEAR_WEBSITES failed: thread panic".into())
                }
            }
        })
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub(super) fn clear_all_websites(
    conn_pool: botcore::shared::utils::DbPool,
    session_id: Uuid,
) -> Result<usize, String> {
    let mut conn = conn_pool
        .get()
        .map_err(|e| format!("Failed to get DB connection: {}", e))?;

    let rows_affected = diesel::sql_query(
        "UPDATE session_website_associations
         SET is_active = false
         WHERE session_id = $1 AND is_active = true",
    )
    .bind::<diesel::sql_types::Uuid, _>(session_id)
    .execute(&mut conn)
    .map_err(|e| format!("Failed to clear websites: {}", e))?;

    Ok(rows_affected)
}
