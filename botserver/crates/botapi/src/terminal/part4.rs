use super::*;

pub async fn terminal_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(query): Query<TerminalQuery>,
) -> impl IntoResponse {
    let session_id = query.session_id.unwrap_or_else(|| {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Time error: {}", e))
            .unwrap_or(std::time::Duration::ZERO)
            .as_millis();
        format!("term-{}", timestamp)
    });

    info!("Terminal WebSocket connection request: {}", session_id);

    ws.on_upgrade(move |socket| handle_terminal_ws(socket, state, session_id))
}
pub(super) async fn handle_terminal_ws(
    socket: WebSocket,
    state: Arc<AppState>,
    session_id: String,
) {
    let (mut sender, mut receiver) = socket.split();

    let terminal_manager = state.terminal_manager.clone();
    let session = match terminal_manager.create_session(&session_id).await {
        Ok(info) => {
            info!("Created terminal session: {:?}", info);
            let welcome = serde_json::json!({
                "type": "connected",
                "session_id": session_id,
                "container": info.container_name,
                "message": "Terminal session created"
            });
            if let Ok(welcome_str) = serde_json::to_string(&welcome) {
                let _ = sender.send(Message::Text(welcome_str)).await;
            }
            terminal_manager.get_session(&session_id).await
        }
        Err(e) => {
            log::error!("Failed to create terminal session: {}", e);
            let error_msg = serde_json::json!({
                "type": "error",
                "message": e
            });
            let _ = sender
                .send(Message::Text(error_msg.to_string()))
                .await;
            return;
        }
    };

    let Some(session_arc) = session else {
        log::error!("Failed to get session after creation");
        return;
    };

    let output_rx = {
        let mut session = session_arc.lock().await;
        match session.take_output_receiver() {
            Some(rx) => rx,
            None => {
                log::error!("Failed to take output receiver");
                return;
            }
        }
    };
    let _session_id_clone = session_id.clone();
    let _terminal_manager_clone = terminal_manager.clone();
    let _session_arc_for_send = session_arc.clone();
    let _session_arc_for_recv = session_arc.clone();

    let mut send_task = tokio::spawn(async move {
        let mut rx = output_rx;
        let mut sender = sender;

        while let Some(output) = rx.recv().await {
            let msg = match output {
                TerminalOutput::Stdout(s) | TerminalOutput::Stderr(s) => {
                    Message::Text(s)
                }
                TerminalOutput::System(s) => {
                    Message::Text(serde_json::json!({
                        "type": "system",
                        "message": s
                    }).to_string())
                }
            };

            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    let session_id_clone2 = session_id.clone();
    let terminal_manager_clone2 = terminal_manager.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Some(session_arc) = terminal_manager_clone2.get_session(&session_id_clone2).await {
                        let trimmed = text.trim();
                        if trimmed.is_empty() {
                            continue;
                        }

                        if trimmed == "\\exit" || trimmed == "exit" {
                            let _ = terminal_manager_clone2.kill_session(&session_id_clone2).await;
                            break;
                        }

                        if trimmed.starts_with("resize ") {
                            let parts: Vec<&str> = trimmed.split_whitespace().collect();
                            if parts.len() >= 3 {
                                if let (Ok(cols), Ok(rows)) = (
                                    parts[1].parse::<u16>(),
                                    parts[2].parse::<u16>(),
                                ) {
                                    let session = session_arc.lock().await;
                                    let _ = session.resize(cols, rows).await;
                                }
                            }
                            continue;
                        }

                        {
                            let session = session_arc.lock().await;
                            if let Err(e) = session.send_command(trimmed).await {
                                log::error!("Failed to send command: {}", e);
                            }
                        }
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(e) => {
                    log::error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => {
            warn!("Terminal send task ended");
        }
        _ = &mut recv_task => {
            info!("Terminal client disconnected");
        }
    }

    if let Err(e) = terminal_manager.kill_session(&session_id).await {
        log::error!("Failed to cleanup terminal session: {}", e);
    }

    info!("Terminal session {} cleaned up", session_id);
}
