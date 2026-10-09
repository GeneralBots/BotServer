use super::*;

pub async fn handle_navigate(
    State(sessions): State<SessionMap>,
    Path(id): Path<String>,
    Json(payload): Json<NavigateRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let session = get_session(&sessions, &id).await?;
    match session.navigate_with_result(&payload.url).await {
        Ok(result) => Ok(Json(serde_json::to_value(result).unwrap_or_default())),
        Err(e) => Err((
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Navigate failed: {e}"),
        )),
    }
}
