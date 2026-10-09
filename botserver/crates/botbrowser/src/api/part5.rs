use super::*;

pub async fn handle_click(
    State(sessions): State<SessionMap>,
    Path(id): Path<String>,
    Json(payload): Json<ClickRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let session = get_session(&sessions, &id).await?;
    match session.click(&payload.selector).await {
        Ok(state) => Ok(Json(serde_json::to_value(state).unwrap_or_default())),
        Err(e) => Err((
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Click failed: {e}"),
        )),
    }
}
