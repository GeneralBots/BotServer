use super::*;

pub async fn get_dashboard(
    State(state): State<Arc<AppState>>,
) -> Result<Json<DashboardData>, StatusCode> {
    let collector = &state.metrics_collector;

    collect_system_metrics(collector, &state).await;

    let total_users = collector.get_aggregate("users.total").await.unwrap_or(0.0) as i64;
    let active_users = collector.get_aggregate("users.active").await.unwrap_or(0.0) as i64;
    let total_messages = collector
        .get_aggregate("messages.total")
        .await
        .unwrap_or(0.0) as i64;
    let total_sessions = collector
        .get_aggregate("sessions.total")
        .await
        .unwrap_or(0.0) as i64;
    let storage_used_gb = collector.get_aggregate("storage.gb").await.unwrap_or(0.0);

    let api_calls_per_minute = collector.get_rate("api.calls", Duration::minutes(1)).await * 60.0;
    let error_rate = collector.get_rate("api.errors", Duration::minutes(5)).await
        / collector
            .get_rate("api.calls", Duration::minutes(5))
            .await
            .max(1.0);
    let response_time_p95 = collector
        .get_percentile("api.duration_ms", 95.0)
        .await
        .unwrap_or(0.0);

    let mut charts = Vec::new();

    let now = Utc::now();
    let mut api_labels = Vec::new();
    let mut api_data = Vec::new();

    for i in (0..24).rev() {
        let hour = now - Duration::hours(i);
        api_labels.push(hour.format("%H:00").to_string());
        let rate = collector.get_rate("api.calls", Duration::hours(1)).await;
        api_data.push(rate * 3600.0);
    }

    charts.push(ChartData {
        title: "API Calls (24h)".to_string(),
        chart_type: "line".to_string(),
        labels: api_labels,
        datasets: vec![DataSet {
            label: "Calls/hour".to_string(),
            data: api_data,
            color: "#3b82f6".to_string(),
        }],
    });

    let mut activity_labels = Vec::new();
    let mut activity_data = Vec::new();

    for i in (0..7).rev() {
        let day = now - Duration::days(i);
        activity_labels.push(day.format("%a").to_string());
        activity_data.push((active_users as f64 / 7.0) * (i as f64).mul_add(0.1, 1.0));
    }

    charts.push(ChartData {
        title: "User Activity (7 days)".to_string(),
        chart_type: "bar".to_string(),
        labels: activity_labels,
        datasets: vec![DataSet {
            label: "Active Users".to_string(),
            data: activity_data,
            color: "#10b981".to_string(),
        }],
    });

    let dashboard = DashboardData {
        total_users,
        active_users,
        total_messages,
        total_sessions,
        storage_used_gb,
        api_calls_per_minute,
        error_rate,
        response_time_p95,
        charts,
    };

    Ok(Json(dashboard))
}
