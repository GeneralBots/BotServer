use super::*;

pub(super) async fn handle_whiteboard_socket(
    socket: WebSocket,
    state: Arc<AppState>,
    whiteboard_id: Uuid,
) {
    let manager = state
        .extensions
        .get::<WhiteboardManager>()
        .await
        .unwrap_or_else(|| Arc::new(WhiteboardManager::new()));

    let receiver = match manager.subscribe(&whiteboard_id).await {
        Some(rx) => rx,
        None => {
            log::error!("Whiteboard {} not found", whiteboard_id);
            return;
        }
    };

    let (mut sender, mut ws_receiver) = socket.split();
    let mut broadcast_receiver = receiver;

    let user_id = Uuid::new_v4();
    let username = format!("User-{}", &user_id.to_string()[..8]);
    let user_color = Color {
        r: (user_id.as_u128() % 200 + 55) as u8,
        g: ((user_id.as_u128() >> 8) % 200 + 55) as u8,
        b: ((user_id.as_u128() >> 16) % 200 + 55) as u8,
        a: 1.0,
    };

    manager
        .user_join(&whiteboard_id, user_id, username.clone(), user_color.clone())
        .await;

    if let Some(whiteboard) = manager.get_whiteboard(&whiteboard_id).await {
        let sync_msg = whiteboard.to_sync_message();
        if let Ok(json) = serde_json::to_string(&sync_msg) {
            let _ = sender.send(Message::Text(json)).await;
        }
    }

    let manager_clone = manager.clone();
    let whiteboard_id_clone = whiteboard_id;

    let send_task = tokio::spawn(async move {
        while let Ok(msg) = broadcast_receiver.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    let receive_task = tokio::spawn(async move {
        while let Some(result) = ws_receiver.next().await {
            match result {
                Ok(Message::Text(text)) => {
                    if let Ok(msg) = serde_json::from_str::<WhiteboardMessage>(&text) {
                        match msg {
                            WhiteboardMessage::Operation { operation, .. } => {
                                let _ = manager_clone
                                    .apply_operation(&whiteboard_id_clone, operation, user_id)
                                    .await;
                            }
                            WhiteboardMessage::CursorMove { mut cursor } => {
                                cursor.user_id = user_id;
                                cursor.username = username.clone();
                                cursor.color = user_color.clone();
                                cursor.last_update = Utc::now();
                                manager_clone.update_cursor(&whiteboard_id_clone, cursor).await;
                            }
                            WhiteboardMessage::Undo { .. } => {
                                if let Some(op) = manager_clone.undo(&whiteboard_id_clone, user_id).await {
                                    manager_clone
                                        .broadcast(
                                            &whiteboard_id_clone,
                                            WhiteboardMessage::Operation {
                                                user_id,
                                                operation: op,
                                            },
                                        )
                                        .await;
                                }
                            }
                            WhiteboardMessage::Redo { .. } => {
                                if let Some(op) = manager_clone.redo(&whiteboard_id_clone, user_id).await {
                                    manager_clone
                                        .broadcast(
                                            &whiteboard_id_clone,
                                            WhiteboardMessage::Operation {
                                                user_id,
                                                operation: op,
                                            },
                                        )
                                        .await;
                                }
                            }
                            WhiteboardMessage::Select { mut selection } => {
                                selection.user_id = user_id;
                                manager_clone
                                    .broadcast(
                                        &whiteboard_id_clone,
                                        WhiteboardMessage::Select { selection },
                                    )
                                    .await;
                            }
                            WhiteboardMessage::Deselect { .. } => {
                                manager_clone
                                    .broadcast(
                                        &whiteboard_id_clone,
                                        WhiteboardMessage::Deselect { user_id },
                                    )
                                    .await;
                            }
                            WhiteboardMessage::RequestSync => {
                                if let Some(whiteboard) =
                                    manager_clone.get_whiteboard(&whiteboard_id_clone).await
                                {
                                    manager_clone
                                        .broadcast(&whiteboard_id_clone, whiteboard.to_sync_message())
                                        .await;
                                }
                            }
                            WhiteboardMessage::Ping => {
                                manager_clone
                                    .broadcast(&whiteboard_id_clone, WhiteboardMessage::Pong)
                                    .await;
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(_) => break,
                _ => {}
            }
        }
    });

    let _ = tokio::join!(send_task, receive_task);

    manager
        .user_leave(&whiteboard_id, user_id)
        .await;
}
