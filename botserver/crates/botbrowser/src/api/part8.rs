use super::*;

pub async fn handle_agent(
    State(sessions): State<SessionMap>,
    Path(id): Path<String>,
    Json(payload): Json<AgentRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let session = get_session(&sessions, &id).await?;
    let session_arc = Arc::new(Mutex::new(session));

    let llm_url = payload.llm_url.unwrap_or_else(|| "http://127.0.0.1:8081/v1/chat/completions".to_string());
    let llm_key = payload.llm_key.unwrap_or_default();
    let llm_model = payload.llm_model.unwrap_or_else(|| "default".to_string());
    let llm_config = agent::LlmConfig::new(&llm_url, &llm_key, &llm_model);
    let max_steps = payload.max_steps.unwrap_or(10);

    match agent::run_agent_loop(&session_arc, &payload.goal, &llm_config, max_steps).await {
        Ok(steps) => Ok(Json(serde_json::json!({
            "steps": steps,
            "total_steps": steps.len()
        }))),
        Err(e) => Err((
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Agent failed: {e}"),
        )),
    }
}
pub async fn handle_agent_ws(
    ws: WebSocketUpgrade,
    State(sessions): State<SessionMap>,
    Path(id): Path<String>,
    Query(params): Query<WsAgentParams>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_agent_ws_stream(socket, sessions, id, params))
}
