use super::*;

/// Get recent user activity for the home page
pub async fn get_recent_activity(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Return recent activity items - in production, fetch from database
    // This powers the home.js loadRecentDocuments() function
    Json(serde_json::json!([
        {
            "id": "1",
            "type": "document",
            "name": "Project Report",
            "path": "/docs/project-report",
            "icon": "📄",
            "modified_at": chrono::Utc::now().to_rfc3339(),
            "app": "docs"
        },
        {
            "id": "2",
            "type": "spreadsheet",
            "name": "Budget 2025",
            "path": "/sheet/budget-2025",
            "icon": "📊",
            "modified_at": (chrono::Utc::now() - chrono::Duration::hours(2)).to_rfc3339(),
            "app": "sheet"
        },
        {
            "id": "3",
            "type": "presentation",
            "name": "Q1 Review",
            "path": "/slides/q1-review",
            "icon": "📽️",
            "modified_at": (chrono::Utc::now() - chrono::Duration::hours(5)).to_rfc3339(),
            "app": "slides"
        },
        {
            "id": "4",
            "type": "folder",
            "name": "Marketing Assets",
            "path": "/drive/marketing",
            "icon": "📁",
            "modified_at": (chrono::Utc::now() - chrono::Duration::days(1)).to_rfc3339(),
            "app": "drive"
        }
    ]))
}

pub fn spawn_metrics_collector(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));

        loop {
            interval.tick().await;

            let collector = &state.metrics_collector;
            collect_system_metrics(collector, &state).await;

            info!("System metrics collected");
        }
    });
}
