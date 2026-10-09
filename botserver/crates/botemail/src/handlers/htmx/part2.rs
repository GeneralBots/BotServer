use super::*;

pub async fn list_labels_htmx(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let user_id = match extract_user_from_session(&headers) {
        Ok(id) => id,
        Err(_) => return axum::response::Html(r#"<div class="label-item">Sign in to view labels</div>"#.to_string()),
    };

    let pool = state.pool.clone();
    let labels_result = tokio::task::spawn_blocking(move || {
        let mut db_conn = pool.get().map_err(|e| format!("DB connection error: {}", e))?;
        diesel::sql_query("SELECT id, name, color FROM email_labels WHERE user_id = $1 ORDER BY name ASC")
            .bind::<diesel::sql_types::Uuid, _>(user_id)
            .get_results::<crate::models::EmailLabelRow>(&mut db_conn)
            .map_err(|e| format!("Failed to load labels: {}", e))
    }).await;

    let labels = match labels_result {
        Ok(Ok(list)) => list,
        _ => Vec::new(),
    };

    if labels.is_empty() {
        return axum::response::Html(r#"<div class="label-item" style="--label-color: #ef4444;"><span class="label-dot" style="background: #ef4444;"></span><span>Important</span></div><div class="label-item" style="--label-color: #3b82f6;"><span class="label-dot" style="background: #3b82f6;"></span><span>Work</span></div><div class="label-item" style="--label-color: #22c55e;"><span class="label-dot" style="background: #22c55e;"></span><span>Personal</span></div>"#.to_string());
    }

    let mut html = String::new();
    use std::fmt::Write;
    for label in labels {
        let _ = write!(html, r##"<div class="label-item" style="--label-color: {};"><span class="label-dot" style="background: {};"></span><span>{}</span></div>"##, label.color, label.color, label.name);
    }

    axum::response::Html(html)
}
