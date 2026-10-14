use super::*;

pub async fn handle_fill(
    State(sessions): State<SessionMap>,
    Path(id): Path<String>,
    Json(payload): Json<FillRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let session = get_session(&sessions, &id).await?;
    match session.fill(&payload.selector, &payload.text).await {
        Ok(()) => Ok(Json(serde_json::json!({ "status": "filled" }))),
        Err(e) => Err((
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Fill failed: {e}"),
        )),
    }
}
pub async fn handle_screenshot(
    State(sessions): State<SessionMap>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let session = get_session(&sessions, &id).await?;
    match session.screenshot().await {
        Ok(data) => {
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
            Ok(Json(serde_json::json!({
                "image_base64": b64,
                "size_bytes": data.len()
            })))
        }
        Err(e) => Err((
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Screenshot failed: {e}"),
        )),
    }
}
pub async fn handle_stream(
    ws: WebSocketUpgrade,
    State(sessions): State<SessionMap>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    ws.on_upgrade(move |ws| async move {
        handle_stream_inner(ws, sessions, id).await;
    })
}
pub(super) async fn handle_stream_inner(mut ws: WebSocket, sessions: SessionMap, id: String) {
    let session = match get_session(&sessions, &id).await {
        Ok(s) => s,
        Err(e) => {
            let _ = ws.send(Message::Text(format!("{{\"error\":\"{}\"}}", e.1))).await;
            return;
        }
    };

    let mut interval = tokio::time::interval(std::time::Duration::from_millis(300));
    loop {
        tokio::select! {
            _ = interval.tick() => {
                match session.screenshot().await {
                    Ok(data) => {
                        let len = data.len();
                        if ws.send(Message::Binary(data)).await.is_err() {
                            break;
                        }
                        log::debug!("Streamed {} bytes for session {}", len, id);
                    }
                    Err(e) => {
                        let msg = format!("{{\"error\":\"screenshot: {e}\"}}");
                        let _ = ws.send(Message::Text(msg)).await;
                        break;
                    }
                }
            }
            Some(msg) = ws.recv() => {
                match msg {
                    Ok(Message::Close(_)) | Err(_) => break,
                    Ok(Message::Text(t))
                        if t == "ping" => {
                            let _ = ws.send(Message::Text("pong".into())).await;
                        }
                    _ => {}
                }
            }
        }
    }
}
pub async fn handle_extract(
    State(sessions): State<SessionMap>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let session = get_session(&sessions, &id).await?;
    let state = session.extract_page_state().await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Extract failed: {e}"),
        )
    })?;
    let links = session.extract_links().await.unwrap_or_default();
    Ok(Json(serde_json::json!({
        "url": state.url,
        "title": state.title,
        "text": state.text_snippet,
        "links": links
    })))
}
