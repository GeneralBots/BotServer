use super::*;

pub async fn handle_query_knowledge_base(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<KbQueryRequest>,
) -> impl IntoResponse {
    let start_time = std::time::Instant::now();

    if payload.query.trim().is_empty() {
        return Json(QueryResponse {
            results: Vec::new(),
            query: payload.query,
            total_results: 0,
            processing_time_ms: 0,
        });
    }

    let conn = state.conn.clone();
    let query = payload.query.clone();
    let top_k = payload.top_k.unwrap_or(5).min(20);
    let min_score = payload.min_score.unwrap_or(0.0);
    let collection = payload.collection.clone();
    let bot_id = resolve_bot_id(&conn);

    let results = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {}", e);
                return Vec::new();
            }
        };

        let search_pattern = format!("%{}%", query.to_lowercase());

        let mut sql = String::from(
            "SELECT
             kc.id as chunk_id,
             kc.content,
             kc.source_id,
             ks.name as source_name,
             kc.chunk_index,
             CAST(ts_rank(to_tsvector('english', kc.content), plainto_tsquery('english', $1)) AS FLOAT4) as score
             FROM knowledge_chunks kc
             JOIN knowledge_sources ks ON kc.source_id = ks.id
             WHERE ks.status = 'indexed'
             AND ks.bot_id = $3",
        );
        sql.push_str(
            " AND (LOWER(kc.content) LIKE $2
             OR to_tsvector('english', kc.content) @@ plainto_tsquery('english', $1))",
        );

        if collection.is_some() {
            sql.push_str(" AND ks.collection = $4");
        }

        let _ = write!(sql, " ORDER BY score DESC, kc.chunk_index ASC LIMIT {top_k}");

        let search_results: Vec<SearchResultRow> = if let Some(ref coll) = collection {
            diesel::sql_query(&sql)
                .bind::<diesel::sql_types::Text, _>(&query)
                .bind::<diesel::sql_types::Text, _>(&search_pattern)
                .bind::<diesel::sql_types::Uuid, _>(bot_id)
                .bind::<diesel::sql_types::Text, _>(coll)
                .load(&mut db_conn)
                .unwrap_or_default()
        } else {
            diesel::sql_query(&sql)
                .bind::<diesel::sql_types::Text, _>(&query)
                .bind::<diesel::sql_types::Text, _>(&search_pattern)
                .bind::<diesel::sql_types::Uuid, _>(bot_id)
                .load(&mut db_conn)
                .unwrap_or_default()
        };

        search_results
            .into_iter()
            .filter(|r| r.score >= min_score)
            .map(|r| KbQueryResult {
                content: r.content,
                source_name: r.source_name,
                source_id: r.source_id,
                chunk_index: r.chunk_index,
                score: r.score,
                metadata: serde_json::json!({}),
            })
            .collect::<Vec<_>>()
    })
    .await
    .unwrap_or_default();

    let processing_time_ms = start_time.elapsed().as_millis() as u64;

    Json(QueryResponse {
        total_results: results.len(),
        results,
        query: payload.query,
        processing_time_ms,
    })
}
pub async fn handle_get_source(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let conn = state.conn.clone();

    let source = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {}", e);
                return None;
            }
        };

        let sources: Vec<KnowledgeSourceRow> = diesel::sql_query(
            "SELECT id, name, source_type, file_path, url, content_hash, chunk_count, status,
             created_at::text, updated_at::text, indexed_at::text
             FROM knowledge_sources WHERE id = $1",
        )
        .bind::<diesel::sql_types::Text, _>(&id)
        .load(&mut db_conn)
        .unwrap_or_default();

        sources.into_iter().next()
    })
    .await
    .unwrap_or(None);

    match source {
        Some(s) => {
            let mut html = String::new();
            html.push_str("<div class=\"source-detail\">");
            html.push_str("<h3>");
            html.push_str(&html_escape(&s.name));
            html.push_str("</h3>");

            html.push_str("<div class=\"detail-grid\">");
            html.push_str("<div class=\"detail-item\"><label>Type:</label><span>");
            html.push_str(&html_escape(&s.source_type));
            html.push_str("</span></div>");

            html.push_str("<div class=\"detail-item\"><label>Status:</label><span>");
            html.push_str(&html_escape(&s.status));
            html.push_str("</span></div>");

            html.push_str("<div class=\"detail-item\"><label>Chunks:</label><span>");
            html.push_str(&s.chunk_count.to_string());
            html.push_str("</span></div>");

            html.push_str("<div class=\"detail-item\"><label>Created:</label><span>");
            html.push_str(&html_escape(&s.created_at));
            html.push_str("</span></div>");

            if let Some(indexed) = &s.indexed_at {
                html.push_str("<div class=\"detail-item\"><label>Indexed:</label><span>");
                html.push_str(&html_escape(indexed));
                html.push_str("</span></div>");
            }

            html.push_str("</div></div>");
            Html(html)
        }
        None => Html("<div class=\"error\">Source not found</div>".to_string()),
    }
}
pub async fn handle_delete_source(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let conn = state.conn.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {}", e);
                return false;
            }
        };

        let _ = diesel::sql_query("DELETE FROM knowledge_chunks WHERE source_id = $1")
            .bind::<diesel::sql_types::Text, _>(&id)
            .execute(&mut db_conn);

        let delete_result = diesel::sql_query("DELETE FROM knowledge_sources WHERE id = $1")
            .bind::<diesel::sql_types::Text, _>(&id)
            .execute(&mut db_conn);

        delete_result.is_ok()
    })
    .await
    .unwrap_or(false);

    if result {
        Html("".to_string())
    } else {
        Html("<div class=\"error\">Failed to delete source</div>".to_string())
    }
}
pub async fn handle_reindex_sources(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ReindexRequest>,
) -> impl IntoResponse {
    let conn = state.conn.clone();
    let source_ids = payload.source_ids.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {}", e);
                return 0;
            }
        };

        if let Some(ids) = source_ids {
            if ids.is_empty() {
                return 0;
            }
            let mut updated = 0usize;
            for id in &ids {
                let affected = diesel::sql_query(
                    "UPDATE knowledge_sources SET status = 'reindexing', updated_at = NOW() WHERE id = $1",
                )
                .bind::<diesel::sql_types::Text, _>(id)
                .execute(&mut db_conn)
                .unwrap_or(0);
                updated += affected;
            }
            updated
        } else {
            diesel::sql_query(
                "UPDATE knowledge_sources SET status = 'reindexing', updated_at = NOW() WHERE status = 'indexed'",
            )
            .execute(&mut db_conn)
            .unwrap_or(0)
        }
    })
    .await
    .unwrap_or(0);

    Json(serde_json::json!({
        "success": true,
        "sources_queued": result,
        "message": format!("{} sources queued for reindexing", result)
    }))
}
