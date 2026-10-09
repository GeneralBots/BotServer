use super::*;

pub async fn handle_get_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let conn = state.conn.clone();
    let bot_id = resolve_bot_id(&conn);

    let stats = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {}", e);
                return serde_json::json!({
                    "total_sources": 0,
                    "total_chunks": 0,
                    "indexed_sources": 0,
                    "pending_sources": 0,
                    "failed_sources": 0
                });
            }
        };

        #[derive(Debug, QueryableByName)]
        #[diesel(check_for_backend(diesel::pg::Pg))]
        struct CountRow {
            #[diesel(sql_type = diesel::sql_types::BigInt)]
            count: i64,
        }

        let total_sources: i64 = diesel::sql_query("SELECT COUNT(*) as count FROM knowledge_sources WHERE bot_id = $1")
            .bind::<diesel::sql_types::Uuid, _>(bot_id)
            .load::<CountRow>(&mut db_conn)
            .map(|v| v.first().map(|r| r.count).unwrap_or(0))
            .unwrap_or(0);

        let total_chunks: i64 = diesel::sql_query(
            "SELECT COUNT(*) as count FROM knowledge_chunks kc JOIN knowledge_sources ks ON kc.source_id = ks.id WHERE ks.bot_id = $1",
        )
            .bind::<diesel::sql_types::Uuid, _>(bot_id)
            .load::<CountRow>(&mut db_conn)
            .map(|v| v.first().map(|r| r.count).unwrap_or(0))
            .unwrap_or(0);

        let indexed: i64 = diesel::sql_query("SELECT COUNT(*) as count FROM knowledge_sources WHERE status = 'indexed' AND bot_id = $1")
            .bind::<diesel::sql_types::Uuid, _>(bot_id)
            .load::<CountRow>(&mut db_conn)
            .map(|v| v.first().map(|r| r.count).unwrap_or(0))
            .unwrap_or(0);

        let pending: i64 = diesel::sql_query(
            "SELECT COUNT(*) as count FROM knowledge_sources WHERE status IN ('pending', 'processing', 'reindexing') AND bot_id = $1",
        )
            .bind::<diesel::sql_types::Uuid, _>(bot_id)
            .load::<CountRow>(&mut db_conn)
            .map(|v| v.first().map(|r| r.count).unwrap_or(0))
            .unwrap_or(0);

        let failed: i64 = diesel::sql_query("SELECT COUNT(*) as count FROM knowledge_sources WHERE status = 'failed' AND bot_id = $1")
            .bind::<diesel::sql_types::Uuid, _>(bot_id)
            .load::<CountRow>(&mut db_conn)
            .map(|v| v.first().map(|r| r.count).unwrap_or(0))
            .unwrap_or(0);

        serde_json::json!({
            "total_sources": total_sources,
            "total_chunks": total_chunks,
            "indexed_sources": indexed,
            "pending_sources": pending,
            "failed_sources": failed
        })
    })
    .await
    .unwrap_or_else(|_| serde_json::json!({
        "error": "Failed to fetch stats"
    }));

    Json(stats)
}
