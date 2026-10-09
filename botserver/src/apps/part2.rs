use super::*;

/// Records an app-store install and returns the resolved app metadata.
pub async fn install_handler(Json(body): Json<InstallBody>) -> Json<serde_json::Value> {
    let app_id = body.app_id.unwrap_or_default();
    let found = registry::all_apps().into_iter().find(|a| a.id == app_id);
    match found {
        Some(app) => {
            let stats = installs();
            let mut counts = match stats.counts.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            let count = counts.entry(app.id.clone()).or_insert(0);
            *count += 1;
            Json(json!({
                "ok": true,
                "app_id": app.id,
                "title": app.title,
                "category": app.category,
                "url": app.url,
                "installs": *count,
            }))
        }
        None => Json(json!({ "ok": false, "error": "unknown_app", "app_id": app_id })),
    }
}
/// Returns the server-side install popularity ranking.
pub async fn install_stats_handler() -> Json<serde_json::Value> {
    let stats = installs();
    let counts = match stats.counts.lock() {
        Ok(g) => g.clone(),
        Err(p) => p.into_inner().clone(),
    };
    let total: u64 = counts.values().sum();
    Json(json!({ "installs": counts, "total": total }))
}
