use super::*;

pub async fn handle_export_citations<S: ResearchState>(
    State(state): State<Arc<S>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let query = params.get("q").cloned().unwrap_or_default();

    if query.trim().is_empty() {
        return Html(
            "<script>alert('No search results to export. Run a search first.');</script>"
                .to_string(),
        );
    }

    let conn = state.db_pool().clone();
    let bot_id = state.bot_id();
    let scope = bot_scope_clause(&bot_id);
    let search_pattern = format!("%{}%", query.to_lowercase());

    let docs = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {e}");
                return Vec::new();
            }
        };

        diesel::sql_query(format!(
            "SELECT id, file_path, collection_name, metadata FROM kb_documents WHERE (LOWER(file_path) LIKE $1 OR metadata::text ILIKE $1){} ORDER BY file_path ASC LIMIT 20",
            scope
        ))
        .bind::<diesel::sql_types::Text, _>(&search_pattern)
        .load::<KbDocumentRow>(&mut db_conn)
        .unwrap_or_default()
    })
    .await
    .unwrap_or_default();

    let mut bibtex = String::new();
    for (i, doc) in docs.iter().enumerate() {
        bibtex.push_str(&format!("@misc{{research_{},\n", i));
        bibtex.push_str(&format!("  title = {{{}}},\n", doc.file_path));
        bibtex.push_str(&format!("  note = {{{}}},\n", doc.collection_name));
        bibtex.push_str("}\n\n");
    }

    Html(format!(
        "<script>navigator.clipboard.writeText({:?}).then(function(){{alert('BibTeX for {} results copied to clipboard');}});</script>",
        bibtex,
        docs.len()
    ))
}
pub async fn handle_sources<S: ResearchState>(
    State(state): State<Arc<S>>,
    axum::extract::Query(params): axum::extract::Query<SearchQuery>,
) -> impl IntoResponse {
    let category = params.q.unwrap_or_default();
    let conn = state.db_pool().clone();
    let bot_id = state.bot_id();
    let scope = bot_scope_clause(&bot_id);

    let sources: Vec<String> = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {}", e);
                return Vec::new();
            }
        };

        if category.is_empty() || category == "all" {
            diesel::sql_query(format!(
                "SELECT DISTINCT collection_name FROM kb_documents WHERE 1 = 1{} ORDER BY collection_name ASC LIMIT 50",
                scope
            ))
            .load::<CollectionNameRow>(&mut db_conn)
            .map(|rows| rows.into_iter().map(|r| r.name).collect())
            .unwrap_or_default()
        } else {
            diesel::sql_query(format!(
                "SELECT DISTINCT collection_name FROM kb_documents WHERE collection_name ILIKE $1{} ORDER BY collection_name ASC LIMIT 50",
                scope
            ))
            .bind::<diesel::sql_types::Text, _>(&format!("%{category}%"))
            .load::<CollectionNameRow>(&mut db_conn)
            .map(|rows| rows.into_iter().map(|r| r.name).collect())
            .unwrap_or_default()
        }
    })
    .await
    .unwrap_or_default();

    Json(serde_json::json!({ "sources": sources }))
}
pub async fn handle_collections_save<S: ResearchState>(
    State(state): State<Arc<S>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let bot_id = state.bot_id().unwrap_or_else(uuid::Uuid::nil);
    let pool = state.db_pool().clone();
    let collection_id = payload
        .get("collection_id")
        .and_then(|v| v.as_str())
        .and_then(|s| uuid::Uuid::parse_str(s).ok());
    let title = payload
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Untitled")
        .to_string();
    let content = payload
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let url = payload
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let res = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| e.to_string())?;
        let id = uuid::Uuid::new_v4();
        diesel::sql_query(
            "INSERT INTO research_collection_items (id, bot_id, collection_id, title, content, url, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, NOW())",
        )
        .bind::<diesel::sql_types::Uuid, _>(id)
        .bind::<diesel::sql_types::Uuid, _>(bot_id)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Uuid>, _>(collection_id)
        .bind::<diesel::sql_types::Text, _>(&title)
        .bind::<diesel::sql_types::Text, _>(&content)
        .bind::<diesel::sql_types::Text, _>(&url)
        .execute(&mut conn)
        .map_err(|e| e.to_string())
    })
    .await;

    match res {
        Ok(Ok(_)) => Json(serde_json::json!({ "ok": true, "saved": true })),
        Ok(Err(e)) => {
            log::error!("research collection save failed: {e}");
            Json(serde_json::json!({ "ok": false, "error": "save_failed" }))
        }
        Err(e) => {
            log::error!("research collection save join failed: {e}");
            Json(serde_json::json!({ "ok": false, "error": "internal" }))
        }
    }
}
pub async fn handle_share<S: ResearchState>(
    State(state): State<Arc<S>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let bot_id = state.bot_id().unwrap_or_else(uuid::Uuid::nil);
    let pool = state.db_pool().clone();
    let title = payload
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Research result")
        .to_string();
    let url = payload
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let channel = payload
        .get("channel")
        .and_then(|v| v.as_str())
        .unwrap_or("link")
        .to_string();
    let id = uuid::Uuid::new_v4();
    let inserted = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| e.to_string())?;
        diesel::sql_query(
            "INSERT INTO research_shares (id, bot_id, title, url, channel, created_at)
             VALUES ($1, $2, $3, $4, $5, NOW())",
        )
        .bind::<diesel::sql_types::Uuid, _>(id)
        .bind::<diesel::sql_types::Uuid, _>(bot_id)
        .bind::<diesel::sql_types::Text, _>(&title)
        .bind::<diesel::sql_types::Text, _>(&url)
        .bind::<diesel::sql_types::Text, _>(&channel)
        .execute(&mut conn)
        .map_err(|e| e.to_string())
    })
    .await;

    match inserted {
        Ok(Ok(_)) => {
            let share_url = format!("/suite/research?share={}", id);
            Json(serde_json::json!({ "ok": true, "share_url": share_url }))
        }
        Ok(Err(e)) => {
            log::error!("research share failed: {e}");
            Json(serde_json::json!({ "ok": false, "error": "share_failed" }))
        }
        Err(e) => {
            log::error!("research share join failed: {e}");
            Json(serde_json::json!({ "ok": false, "error": "internal" }))
        }
    }
}
pub(super) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
