use super::*;

pub(super) async fn handle_docs_connection(socket: WebSocket, doc_id: String, auth: Option<(String, String)>) {
    let (mut sender, mut receiver) = socket.split();

    let channels = get_collab_channels();
    let broadcast_tx = {
        let mut channels_write = channels.write().await;
        channels_write
            .entry(doc_id.clone())
            .or_insert_with(|| broadcast::channel(100).0)
            .clone()
    };

    let mut broadcast_rx = broadcast_tx.subscribe();

    let (user_id, user_name) = match auth {
        Some((id, name)) => (id, name),
        None => {
            let id = uuid::Uuid::new_v4().to_string();
            (id.clone(), format!("Guest {}", &id[..8]))
        }
    };
    let user_id_for_send = user_id.clone();
    let user_color = get_random_color();

    {
        let mut presence = get_presence().write().await;
        let users = presence.entry(doc_id.clone()).or_default();
        users.push(UserPresence {
            user_id: user_id.clone(),
            user_name: user_name.clone(),
            user_color: user_color.clone(),
            cursor_position: None,
            last_active: Utc::now(),
            status: "active".to_string(),
        });
    }

    let join_msg = CollabMessage {
        msg_type: "join".to_string(),
        doc_id: doc_id.clone(),
        user_id: user_id.clone(),
        user_name: user_name.clone(),
        user_color: user_color.clone(),
        position: None,
        length: None,
        content: None,
        format: None,
        timestamp: Utc::now(),
    };

    if let Err(e) = broadcast_tx.send(join_msg) {
        log::error!("Failed to broadcast join: {}", e);
    }

    let broadcast_tx_clone = broadcast_tx.clone();
    let user_id_clone = user_id.clone();
    let doc_id_clone = doc_id.clone();
    let user_name_clone = user_name.clone();
    let user_color_clone = user_color.clone();

    let receive_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(mut collab_msg) = serde_json::from_str::<CollabMessage>(&text) {
                        collab_msg.user_id = user_id_clone.clone();
                        collab_msg.user_name = user_name_clone.clone();
                        collab_msg.user_color = user_color_clone.clone();
                        collab_msg.doc_id = doc_id_clone.clone();
                        collab_msg.timestamp = Utc::now();

                        match collab_msg.msg_type.as_str() {
                            "cursor" => {
                                let mut presence = get_presence().write().await;
                                if let Some(users) = presence.get_mut(&doc_id_clone) {
                                    for user in users.iter_mut() {
                                        if user.user_id == user_id_clone {
                                            user.cursor_position = collab_msg.position;
                                            user.last_active = Utc::now();
                                        }
                                    }
                                }
                            }
                            "typing_start" => {
                                if let Some(pos) = collab_msg.position {
                                    let mut typing = get_typing().write().await;
                                    let indicators =
                                        typing.entry(doc_id_clone.clone()).or_default();
                                    indicators.retain(|t| t.user_id != user_id_clone);
                                    indicators.push(TypingIndicator {
                                        user_id: user_id_clone.clone(),
                                        user_name: user_name_clone.clone(),
                                        position: pos,
                                        started_at: Utc::now(),
                                    });
                                }
                            }
                            "typing_stop" => {
                                let mut typing = get_typing().write().await;
                                if let Some(indicators) = typing.get_mut(&doc_id_clone) {
                                    indicators.retain(|t| t.user_id != user_id_clone);
                                }
                            }
                            "selection" => {
                                if let Some(content) = &collab_msg.content {
                                    if let Ok(sel_data) =
                                        serde_json::from_str::<serde_json::Value>(content)
                                    {
                                        let mut selections = get_selections().write().await;
                                        let sels =
                                            selections.entry(doc_id_clone.clone()).or_default();
                                        sels.retain(|s| s.user_id != user_id_clone);

                                        if let (Some(start), Some(end)) = (
                                            sel_data.get("start").and_then(|v| v.as_u64()),
                                            sel_data.get("end").and_then(|v| v.as_u64()),
                                        ) {
                                            sels.push(SelectionInfo {
                                                user_id: user_id_clone.clone(),
                                                user_name: user_name_clone.clone(),
                                                user_color: user_color_clone.clone(),
                                                start_position: start as usize,
                                                end_position: end as usize,
                                            });
                                        }
                                    }
                                }
                            }
                            "mention" => {
                                if let Some(content) = &collab_msg.content {
                                    if let Ok(mention_data) =
                                        serde_json::from_str::<serde_json::Value>(content)
                                    {
                                        if let (Some(to_user), Some(message)) = (
                                            mention_data
                                                .get("to_user_id")
                                                .and_then(|v| v.as_str()),
                                            mention_data
                                                .get("message")
                                                .and_then(|v| v.as_str()),
                                        ) {
                                            let mut mentions = get_mentions().write().await;
                                            let user_mentions =
                                                mentions.entry(to_user.to_string()).or_default();
                                            user_mentions.push(MentionNotification {
                                                id: uuid::Uuid::new_v4().to_string(),
                                                doc_id: doc_id_clone.clone(),
                                                from_user_id: user_id_clone.clone(),
                                                from_user_name: user_name_clone.clone(),
                                                to_user_id: to_user.to_string(),
                                                position: collab_msg.position.unwrap_or(0),
                                                message: message.to_string(),
                                                created_at: Utc::now(),
                                                read: false,
                                            });
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }

                        if let Err(e) = broadcast_tx_clone.send(collab_msg) {
                            log::error!("Failed to broadcast message: {}", e);
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

    let send_task = tokio::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            if msg.user_id == user_id_for_send {
                continue;
            }
            if let Ok(json) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    let doc_id_leave = doc_id.clone();
    let user_id_leave = user_id.clone();

    let leave_msg = CollabMessage {
        msg_type: "leave".to_string(),
        doc_id: doc_id.clone(),
        user_id: user_id.clone(),
        user_name,
        user_color,
        position: None,
        length: None,
        content: None,
        format: None,
        timestamp: Utc::now(),
    };

    tokio::select! {
        _ = receive_task => {}
        _ = send_task => {}
    }

    {
        let mut presence = get_presence().write().await;
        if let Some(users) = presence.get_mut(&doc_id_leave) {
            users.retain(|u| u.user_id != user_id_leave);
        }
    }

    {
        let mut typing = get_typing().write().await;
        if let Some(indicators) = typing.get_mut(&doc_id_leave) {
            indicators.retain(|t| t.user_id != user_id_leave);
        }
    }

    {
        let mut selections = get_selections().write().await;
        if let Some(sels) = selections.get_mut(&doc_id_leave) {
            sels.retain(|s| s.user_id != user_id_leave);
        }
    }

    if let Err(e) = broadcast_tx.send(leave_msg) {
        info!("User left (broadcast may have no receivers): {}", e);
    }
}
pub async fn broadcast_doc_change(
    doc_id: &str,
    user_id: &str,
    user_name: &str,
    position: Option<usize>,
    content: Option<&str>,
) {
    let channels = get_collab_channels().read().await;
    if let Some(tx) = channels.get(doc_id) {
        let msg = CollabMessage {
            msg_type: "edit".to_string(),
            doc_id: doc_id.to_string(),
            user_id: user_id.to_string(),
            user_name: user_name.to_string(),
            user_color: get_random_color(),
            position,
            length: None,
            content: content.map(|s| s.to_string()),
            format: None,
            timestamp: Utc::now(),
        };
        let _ = tx.send(msg);
    }
}
pub async fn mark_mention_read(user_id: &str, mention_id: &str) {
    let mut mentions = get_mentions().write().await;
    if let Some(user_mentions) = mentions.get_mut(user_id) {
        for mention in user_mentions.iter_mut() {
            if mention.id == mention_id {
                mention.read = true;
            }
        }
    }
}
pub async fn clear_user_mentions(user_id: &str) {
    let mut mentions = get_mentions().write().await;
    mentions.remove(user_id);
}
pub(super) fn get_random_color() -> String {
    use rand::Rng;
    let colors = [
        "#FF6B6B", "#4ECDC4", "#45B7D1", "#96CEB4", "#FFEAA7", "#DDA0DD",
        "#98D8C8", "#F7DC6F", "#BB8FCE", "#85C1E9", "#F1948A", "#82E0AA",
        "#F8C471", "#AED6F1", "#D7BDE2",
    ];
    let idx = rand::rng().random_range(0..colors.len());
    colors[idx].to_string()
}
