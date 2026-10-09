use super::*;

pub(super) async fn handle_agent_ws_stream(
    mut socket: WebSocket,
    sessions: SessionMap,
    id: String,
    params: WsAgentParams,
) {
    let session = match get_session(&sessions, &id).await {
        Ok(s) => s,
        Err(e) => {
            let _ = socket
                .send(Message::Text(
                    serde_json::json!({ "error": e.1 }).to_string(),
                ))
                .await;
            return;
        }
    };

    let session_arc = Arc::new(Mutex::new(session));

    let llm_url = params.llm_url.clone().unwrap_or_else(|| "http://127.0.0.1:8081/v1/chat/completions".to_string());
    let llm_key = params.llm_key.clone().unwrap_or_default();
    let llm_model = params.llm_model.clone().unwrap_or_else(|| "default".to_string());
    let llm_config = agent::LlmConfig::new(&llm_url, &llm_key, &llm_model);

    let max_steps = params.max_steps.unwrap_or(10);
    let mut step_count = 0;

    let _ = socket
        .send(Message::Text(
            serde_json::json!({ "type": "start", "goal": params.goal }).to_string(),
        ))
        .await;

    while step_count < max_steps {
        step_count += 1;

        let observation = match agent::observe_page(&session_arc).await {
            Ok(o) => o,
            Err(e) => {
                let _ = socket
                    .send(Message::Text(
                        serde_json::json!({ "type": "error", "message": e.to_string() }).to_string(),
                    ))
                    .await;
                break;
            }
        };

        let _ = socket
            .send(Message::Text(
                serde_json::json!({
                    "type": "observation",
                    "step": step_count,
                    "url": observation.url,
                    "title": observation.title,
                    "visible_text": observation.visible_text.chars().take(2000).collect::<String>(),
                    "links_count": observation.links.len()
                })
                .to_string(),
            ))
            .await;

        let action = match agent::decide_next_action(
            &observation,
            &params.goal,
            None,
            &llm_config,
        )
        .await
        {
            Ok(a) => a,
            Err(e) => {
                let _ = socket
                    .send(Message::Text(
                        serde_json::json!({ "type": "error", "message": e.to_string() }).to_string(),
                    ))
                    .await;
                break;
            }
        };

        let _ = socket
            .send(Message::Text(
                serde_json::json!({
                    "type": "action",
                    "step": step_count,
                    "reasoning": action.reasoning,
                    "action": format!("{:?}", action.action)
                })
                .to_string(),
            ))
            .await;

        match &action.action {
            agent::AgentAction::Done { summary } => {
                let _ = socket
                    .send(Message::Text(
                        serde_json::json!({
                            "type": "done",
                            "summary": summary,
                            "total_steps": step_count
                        })
                        .to_string(),
                    ))
                    .await;
                return;
            }
            _ => {
                if let Err(e) = agent::execute_action(&session_arc, &action.action).await {
                    let _ = socket
                        .send(Message::Text(
                            serde_json::json!({ "type": "action_error", "error": e.to_string() }).to_string(),
                        ))
                        .await;
                }
            }
        }
    }

    let _ = socket
        .send(Message::Text(
            serde_json::json!({
                "type": "done",
                "summary": "Max steps reached",
                "total_steps": step_count
            })
            .to_string(),
        ))
        .await;
}
pub async fn handle_close(
    State(sessions): State<SessionMap>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let session = get_session(&sessions, &id).await?;
    session.close().await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Close failed: {e}"),
        )
    })?;
    let mut map = sessions.lock().await;
    map.remove(&id);
    log::info!("Browser session closed: {id}");
    Ok(Json(serde_json::json!({ "status": "closed", "id": id })))
}
