use super::*;

impl TeamsAdapter {
    pub(super) fn new() -> Self { Self }
    pub(crate) async fn send_message(&self, _response: botcore::shared::BotResponse) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Err("Teams adapter not available - register at botserver layer".into())
    }
}
pub fn register_universal_messaging(state: Arc<AppState>, user: UserSession, engine: &mut Engine) {
    register_send_file_to(state.clone(), user.clone(), engine);
    register_send_to(state.clone(), user.clone(), engine);
    register_broadcast(state, user, engine);
}

// TALK TO lives in hear_talk.rs (`talk_keyword`): registering it here as well
// collided with the TALK syntax.
pub(super) fn register_send_file_to(state: Arc<AppState>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_arc = Arc::new(user);

    let user_clone = Arc::clone(&user_arc);
    if let Err(e) = engine
        .register_custom_syntax(
            ["SEND", "FILE", "TO", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let recipient = context.eval_expression_tree(&inputs[0])?.to_string();
                let file = context.eval_expression_tree(&inputs[1])?;

                trace!("SEND FILE TO: Sending file to {}", recipient);

                let state_for_send = Arc::clone(&state_clone);
                let user_for_send = Arc::clone(&user_clone);

                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build();
                    let result = match rt {
                        Ok(rt) => rt.block_on(async {
                            send_file_to_recipient(state_for_send, &user_for_send, &recipient, file)
                                .await
                        }),
                        Err(_) => Err("Failed to create runtime".into()),
                    };
                    let _ = tx.send(result);
                });
                rx.recv().unwrap_or(Err("Failed to receive result".into()))
                .map_err(|e| format!("Failed to send file: {}", e))?;

                Ok(Dynamic::UNIT)
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    let state_clone2 = Arc::clone(&state);
    let user_clone2 = Arc::clone(&user_arc);

    if let Err(e) = engine
        .register_custom_syntax(
            ["SEND", "FILE", "TO", "$expr$", ",", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let recipient = context.eval_expression_tree(&inputs[0])?.to_string();
                let file = context.eval_expression_tree(&inputs[1])?;
                let caption = context.eval_expression_tree(&inputs[2])?.to_string();

                trace!("SEND FILE TO: Sending file with caption to {}", recipient);

                let state_for_send = Arc::clone(&state_clone2);
                let user_for_send = Arc::clone(&user_clone2);

                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build();
                    let result = match rt {
                        Ok(rt) => rt.block_on(async {
                            send_file_with_caption_to_recipient(
                                state_for_send,
                                &user_for_send,
                                &recipient,
                                file,
                                &caption,
                            )
                            .await
                        }),
                        Err(_) => Err("Failed to create runtime".into()),
                    };
                    let _ = tx.send(result);
                });
                rx.recv().unwrap_or(Err("Failed to receive result".into()))
                .map_err(|e| format!("Failed to send file with caption: {}", e))?;

                Ok(Dynamic::UNIT)
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub(super) fn register_send_to(state: Arc<AppState>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();

    if let Err(e) = engine
        .register_custom_syntax(
            ["SEND", "TO", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let target = context.eval_expression_tree(&inputs[0])?.to_string();
                let message = context.eval_expression_tree(&inputs[1])?.to_string();

                trace!("SEND TO: {} with message", target);

                let state_for_send = Arc::clone(&state_clone);
                let user_for_send = user_clone.clone();

                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build();
                    let result = match rt {
                        Ok(rt) => rt.block_on(async {
                            send_to_specific_channel(state_for_send, &user_for_send, &target, &message)
                                .await
                        }),
                        Err(_) => Err("Failed to create runtime".into()),
                    };
                    let _ = tx.send(result);
                });
                rx.recv().unwrap_or(Err("Failed to receive result".into()))
                .map_err(|e| format!("Failed to send: {}", e))?;

                Ok(Dynamic::UNIT)
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    // Also register send_to as regular function for .ast compatibility
    let state_fn = Arc::clone(&state);
    let user_fn = user.clone();
    engine.register_fn("send_to", move |target: String, message: String| -> Dynamic {
        trace!("send_to function: target={}, message={}", target, message);

        let state_for_send = Arc::clone(&state_fn);
        let user_for_send = user_fn.clone();
        let target_clone = target.clone();
        let message_clone = message.clone();

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            let result = match rt {
                Ok(rt) => rt.block_on(async {
                    send_to_specific_channel(state_for_send, &user_for_send, &target_clone, &message_clone)
                        .await
                }),
                Err(_) => Err("Failed to create runtime".into()),
            };
            let _ = tx.send(result);
        });
        match rx.recv() {
            Ok(Ok(())) => Dynamic::UNIT,
            Ok(Err(e)) => Dynamic::from(format!("Failed to send: {}", e)),
            Err(e) => Dynamic::from(format!("Failed to receive send result: {}", e)),
        }
    });
}
pub(super) fn register_broadcast(state: Arc<AppState>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);

    if let Err(e) = engine
        .register_custom_syntax(
            ["BROADCAST", "$expr$", "TO", "$expr$"],
            false,
            move |context, inputs| {
                let message = context.eval_expression_tree(&inputs[0])?.to_string();
                let recipients = context.eval_expression_tree(&inputs[1])?;

                trace!("BROADCAST: Sending to multiple recipients");

                let state_for_send = Arc::clone(&state_clone);
                let user_for_send = user.clone();

                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build();
                    let result: Result<Dynamic, String> = match rt {
                        Ok(rt) => rt.block_on(async {
                            broadcast_message(state_for_send, &user_for_send, &message, recipients)
                                .await
                                .map_err(|e| format!("{}", e))
                        }),
                        Err(_) => Err("Failed to create runtime".into()),
                    };
                    let _ = tx.send(result);
                });
                let results = rx.recv().unwrap_or(Err("Failed to receive result".into()))
                    .map_err(|e| format!("Failed to broadcast: {}", e))?;

                Ok(results)
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub async fn send_message_to_recipient(
    state: Arc<AppState>,
    user: &UserSession,
    recipient: &str,
    message: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (channel, recipient_id) = parse_recipient(state.clone(), recipient).await?;

    match channel.as_str() {
        "whatsapp" => {
            let adapter = WhatsAppAdapter::new(&state, user.bot_id);
            let response = botcore::shared::BotResponse {
                bot_id: "default".to_string(),
                session_id: user.id.to_string(),
                user_id: recipient_id.clone(),
                channel: "whatsapp".to_string(),
                content: message.to_string(),
                message_type: MessageType::EXTERNAL,
                stream_token: None,
                is_complete: true,
            suggestions: vec![],
            switchers: Vec::new(),
            context_name: None,
            context_length: 0,
            context_max_length: 0,
            reasoning: String::new(),
        };
        adapter.send_message(response).await?;
    }
    "instagram" => {
        let adapter = InstagramAdapter::new();
        let response = botcore::shared::BotResponse {
            bot_id: "default".to_string(),
            session_id: user.id.to_string(),
            user_id: recipient_id.clone(),
            channel: "instagram".to_string(),
            content: message.to_string(),
            message_type: MessageType::EXTERNAL,
            stream_token: None,
            is_complete: true,
            suggestions: vec![],
            switchers: Vec::new(),
            context_name: None,
            context_length: 0,
            context_max_length: 0,
            reasoning: String::new(),
        };
        adapter.send_message(response).await?;
    }
    "teams" => {
        let adapter = TeamsAdapter::new();
        let response = botcore::shared::BotResponse {
            bot_id: "default".to_string(),
            session_id: user.id.to_string(),
            user_id: recipient_id.clone(),
            channel: "teams".to_string(),
            content: message.to_string(),
            message_type: MessageType::EXTERNAL,
            stream_token: None,
            is_complete: true,
            suggestions: vec![],
            switchers: Vec::new(),
                context_name: None,
                context_length: 0,
                context_max_length: 0,
            reasoning: String::new(),
            };
            adapter.send_message(response).await?;
        }
        "web" => {
            send_web_message(state.clone(), &recipient_id, message).await?;
        }
        "email" => {
            send_email(state.clone(), user.bot_id, &recipient_id, message)?;
        }
        _ => {
            log::error!("Unknown channel: {}", channel);
            return Err(format!("Unknown channel: {}", channel).into());
        }
    }

    Ok(())
}
