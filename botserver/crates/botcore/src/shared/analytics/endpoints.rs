use super::*;

pub async fn get_metric(
    State(state): State<Arc<AppState>>,
    Query(query): Query<MetricQuery>,
) -> Json<serde_json::Value> {
    let collector = &state.metrics_collector;

    let result = match query.aggregation.as_deref() {
        Some("p50") => collector.get_percentile(&query.name, 50.0).await,
        Some("p95") => collector.get_percentile(&query.name, 95.0).await,
        Some("p99") => collector.get_percentile(&query.name, 99.0).await,
        Some("rate") => {
            let window = Duration::minutes(query.window_minutes.unwrap_or(1));
            Some(collector.get_rate(&query.name, window).await)
        }
        Some("sum" | _) | None => collector.get_aggregate(&query.name).await,
    };

    Json(match result {
        Some(value) => serde_json::json!({
            "metric": query.name,
            "value": value,
            "timestamp": Utc::now(),
        }),
        None => serde_json::json!({
            "error": "Metric not found",
            "metric": query.name,
        }),
    })
}

pub async fn export_metrics(State(state): State<Arc<AppState>>) -> (StatusCode, String) {
    let collector = &state.metrics_collector;
    let metrics = collector.get_metrics().await;

    use std::fmt::Write;
    let mut prometheus_format = String::new();
    let mut seen_metrics = HashMap::new();

    for metric in metrics {
        if !seen_metrics.contains_key(&metric.name) {
            let _ = writeln!(prometheus_format, "# TYPE {} gauge", metric.name);
            seen_metrics.insert(metric.name.clone(), true);
        }

        let labels = metric
            .labels
            .iter()
            .map(|(k, v)| format!("{}=\"{}\"", k, v))
            .collect::<Vec<_>>()
            .join(",");

        if labels.is_empty() {
            let _ = writeln!(
                prometheus_format,
                "{} {} {}",
                metric.name,
                metric.value,
                metric.timestamp.timestamp_millis()
            );
        } else {
            let _ = writeln!(
                prometheus_format,
                "{}{{{}}} {} {}",
                metric.name,
                labels,
                metric.value,
                metric.timestamp.timestamp_millis()
            );
        }
    }

    (StatusCode::OK, prometheus_format)
}

pub fn configure() -> axum::routing::Router<Arc<AppState>> {
    use axum::routing::{get, Router};

    Router::new()
        .route(ApiUrls::ANALYTICS_DASHBOARD, get(get_dashboard))
        .route(ApiUrls::ANALYTICS_METRIC, get(get_metric))
        .route(ApiUrls::METRICS, get(export_metrics))
        .route("/api/activity/recent", get(get_recent_activity))
}
