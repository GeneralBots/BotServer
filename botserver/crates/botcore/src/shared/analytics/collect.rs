use super::*;

pub async fn collect_system_metrics(collector: &MetricsCollector, state: &AppState) {
    let mut conn = match state.conn.get() {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to get database connection for metrics: {}", e);
            return;
        }
    };

    #[derive(QueryableByName)]
    struct CountResult {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        count: i64,
    }

    let total_users: i64 = diesel::sql_query("SELECT COUNT(*) as count FROM users")
        .get_result::<CountResult>(&mut conn)
        .map(|r| r.count)
        .unwrap_or(0);

    let active_cutoff = Utc::now() - Duration::days(7);
    let active_users: i64 = diesel::sql_query(
        "SELECT COUNT(DISTINCT user_id) as count FROM user_sessions WHERE updated_at > $1",
    )
    .bind::<diesel::sql_types::Timestamptz, _>(active_cutoff)
    .get_result::<CountResult>(&mut conn)
    .map(|r| r.count)
    .unwrap_or(0);

    let total_sessions: i64 = diesel::sql_query("SELECT COUNT(*) as count FROM user_sessions")
        .get_result::<CountResult>(&mut conn)
        .map(|r| r.count)
        .unwrap_or(0);

    #[derive(QueryableByName)]
    struct SizeResult {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        total_size: i64,
    }

    let storage_bytes: i64 =
        diesel::sql_query("SELECT COALESCE(SUM(file_size), 0) as total_size FROM kb_documents")
            .get_result::<SizeResult>(&mut conn)
            .map(|r| r.total_size)
            .unwrap_or(0);

    let storage_gb = storage_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

    let mut labels = HashMap::new();
    labels.insert("source".to_string(), "system".to_string());

    collector
        .gauge(
            "users.total".to_string(),
            total_users as f64,
            labels.clone(),
        )
        .await;
    collector
        .gauge(
            "users.active".to_string(),
            active_users as f64,
            labels.clone(),
        )
        .await;
    collector
        .gauge(
            "sessions.total".to_string(),
            total_sessions as f64,
            labels.clone(),
        )
        .await;
    collector
        .gauge("storage.gb".to_string(), storage_gb, labels.clone())
        .await;
}

pub async fn track_api_call(
    collector: &MetricsCollector,
    endpoint: String,
    duration_ms: f64,
    status: u16,
) {
    let mut labels = HashMap::new();
    labels.insert("endpoint".to_string(), endpoint);
    labels.insert("status".to_string(), status.to_string());

    collector
        .increment("api.calls".to_string(), labels.clone())
        .await;
    collector
        .record("api.duration_ms".to_string(), duration_ms, labels.clone())
        .await;

    if status >= 500 {
        collector.increment("api.errors".to_string(), labels).await;
    }
}

pub async fn track_message(collector: &MetricsCollector, channel: String, user_id: String) {
    let mut labels = HashMap::new();
    labels.insert("channel".to_string(), channel);
    labels.insert("user_id".to_string(), user_id);

    collector
        .increment("messages.total".to_string(), labels)
        .await;
}

pub async fn track_file_operation(
    collector: &MetricsCollector,
    operation: String,
    size_bytes: i64,
    success: bool,
) {
    let mut labels = HashMap::new();
    labels.insert("operation".to_string(), operation);
    labels.insert("success".to_string(), success.to_string());

    collector
        .increment("files.operations".to_string(), labels.clone())
        .await;

    if success {
        collector
            .record("files.bytes".to_string(), size_bytes as f64, labels)
            .await;
    }
}
