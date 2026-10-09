use super::*;

pub async fn create_session(
    State(sessions): State<SessionMap>,
    Json(payload): Json<CreateSessionRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let headless = payload.headless.unwrap_or(true);
    match BrowserSession::new(headless).await {
        Ok(session) => {
            let id = session.id.clone();
            sessions.lock().await.insert(id.clone(), session);
            log::info!("Browser session created: {id}");
            Ok(Json(serde_json::json!({
                "id": id,
                "status": "created",
                "headless": headless
            })))
        }
        Err(e) => {
            log::error!("Failed to create browser session: {e}");
            Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create browser: {e}"),
            ))
        }
    }
}
pub(super) async fn get_session(
    sessions: &SessionMap,
    id: &str,
) -> Result<BrowserSession, (axum::http::StatusCode, String)> {
    let map = sessions.lock().await;
    map.get(id).cloned().ok_or_else(|| {
        (
            axum::http::StatusCode::NOT_FOUND,
            format!("Session {id} not found"),
        )
    })
}
