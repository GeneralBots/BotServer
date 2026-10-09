use super::*;

pub async fn create_terminal(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateTerminalRequest>,
) -> impl IntoResponse {
    let session_id = payload.session_id.unwrap_or_else(|| {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Time error: {}", e))
            .unwrap_or(std::time::Duration::ZERO)
            .as_millis();
        format!("term-{}", timestamp)
    });

    match state.terminal_manager.create_session(&session_id).await {
        Ok(info) => (
            axum::http::StatusCode::CREATED,
            Json(serde_json::json!({
                "success": true,
                "terminal": info
            })),
        ),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "error": e
            })),
        ),
    }
}
pub async fn kill_terminal(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let session_id = payload
        .get("session_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if session_id.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "success": false,
                "error": "session_id is required"
            })),
        );
    }

    match state.terminal_manager.kill_session(session_id).await {
        Ok(()) => (
            axum::http::StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": "Terminal session killed"
            })),
        ),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "error": e
            })),
        ),
    }
}
pub async fn list_terminals(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let terminals = state.terminal_manager.list_sessions().await;
    Json(serde_json::json!({
        "terminals": terminals
    }))
}
