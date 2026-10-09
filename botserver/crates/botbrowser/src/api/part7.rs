use super::*;

pub async fn handle_execute(
    State(sessions): State<SessionMap>,
    Path(id): Path<String>,
    Json(payload): Json<ExecuteRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let session = get_session(&sessions, &id).await?;
    match session.execute(&payload.script).await {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err((
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Execute failed: {e}"),
        )),
    }
}
pub async fn handle_state(
    State(sessions): State<SessionMap>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let session = get_session(&sessions, &id).await?;
    match session.extract_page_state().await {
        Ok(state) => Ok(Json(serde_json::to_value(state).unwrap_or_default())),
        Err(e) => Err((
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Get state failed: {e}"),
        )),
    }
}
