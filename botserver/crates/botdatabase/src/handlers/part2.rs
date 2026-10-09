use super::*;

/// #1386 — the bot database connection honoring the `X-Db-Env` selector.
/// When `X-Vibe-Project` is present the connection targets the selected Vibe
/// project's OWN database pair (same contextualization as the Chat/Terminal
/// panes); otherwise it targets the bot context database.
pub(super) fn get_bot_conn_for_env(
    state: &AppState,
    bot_id: uuid::Uuid,
    headers: &HeaderMap,
) -> Result<diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::PgConnection>>, (StatusCode, Json<serde_json::Value>)> {
    let env = headers
        .get("X-Db-Env")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("production");
    let pool = if let Some(project) = headers
        .get("X-Vibe-Project")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| uuid::Uuid::parse_str(v).ok())
    {
        resolve_project_pool(project, headers)?
    } else {
        get_bot_pool_for_env(state, bot_id, env)?
    };
    pool.get()
        .map_err(|e| internal_error(&format!("Database connection error: {e}")))
}
pub(super) async fn get_bot_database_url(
    state: &AppState,
    bot_id: uuid::Uuid,
) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    if let Some(cached) = bot_db_url_cache().lock().map(|c| c.get(&bot_id).cloned()).unwrap_or(None) {
        return Ok(cached);
    }

    let db_name: String = {
        let pool = db::pool().map_err(|(code, msg)| (code, Json(serde_json::json!({"error": msg}))))?;
        let mut conn = pool.get().map_err(|e| internal_error(&format!("Main DB connection error: {e}")))?;

        #[derive(QueryableByName, Debug)]
        struct DbName {
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
            database_name: Option<String>,
        }

        let result: Option<DbName> = sql_query("SELECT database_name::text FROM bots WHERE id = $1 AND is_active = true")
            .bind::<diesel::sql_types::Uuid, _>(bot_id)
            .get_result(&mut conn)
            .ok();

        result.and_then(|r| r.database_name).ok_or_else(|| error_response("Bot database not configured"))?
    };

    // The bot's OWN database is always used. Falling back to the platform's
    // main database here (as was previously done for empty per-bot databases)
    // leaked the internal botserver schema into the DB dialog.
    let base_url = state.database_url.clone();
    let base = base_url
        .rfind('/')
        .map(|pos| &base_url[..pos])
        .unwrap_or(&base_url);
    let bot_db_url = format!("{base}/{db_name}");

    if let Ok(mut cache) = bot_db_url_cache().lock() {
        cache.insert(bot_id, bot_db_url.clone());
    }
    Ok(bot_db_url)
}
