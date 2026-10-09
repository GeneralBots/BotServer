use super::*;

pub fn send_mail_keyword(state: Arc<AppState>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();

    if let Err(e) = engine
        .register_custom_syntax(
            [
                "SEND", "MAIL", "$expr$", ",", "$expr$", ",", "$expr$", ",", "$expr$",
            ],
            false,
            move |context, inputs| {
                let to = context.eval_expression_tree(&inputs[0])?.to_string();
                let subject = context.eval_expression_tree(&inputs[1])?.to_string();
                let body = context.eval_expression_tree(&inputs[2])?.to_string();
                let attachments_input = context.eval_expression_tree(&inputs[3])?;

                let mut attachments = Vec::new();
                if attachments_input.is_array() {
                    let arr = attachments_input.cast::<rhai::Array>();
                    for item in arr.iter() {
                        attachments.push(item.to_string());
                    }
                } else if !attachments_input.to_string().is_empty() {
                    attachments.push(attachments_input.to_string());
                }

                trace!(
                    "SEND MAIL: to={}, subject={}, attachments={:?} for user={}",
                    to,
                    subject,
                    attachments,
                    user_clone.user_id
                );

                let state_for_task = Arc::clone(&state_clone);
                let user_for_task = user_clone.clone();

                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    let send_err = if let Ok(rt) = rt {
                        let result = rt.block_on(async move {
                            execute_send_mail(
                                &state_for_task,
                                &user_for_task,
                                &to,
                                &subject,
                                &body,
                                attachments,
                                None,
                            )
                            .await
                        });
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".to_string()))
                            .err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send SEND MAIL result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                    Ok(Ok(message_id)) => Ok(Dynamic::from(message_id)),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("SEND MAIL failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "SEND MAIL timed out".into(),
                            rhai::Position::NONE,
                        )))
                    }
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("SEND MAIL thread failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    let state_clone2 = Arc::clone(&state);
    let user_clone2 = user.clone();

    if let Err(e) = engine
        .register_custom_syntax(
            [
                "SEND", "MAIL", "$expr$", ",", "$expr$", ",", "$expr$", "USING", "$expr$",
            ],
            false,
            move |context, inputs| {
                let to = context.eval_expression_tree(&inputs[0])?.to_string();
                let subject = context.eval_expression_tree(&inputs[1])?.to_string();
                let body = context.eval_expression_tree(&inputs[2])?.to_string();
                let using_account = context.eval_expression_tree(&inputs[3])?.to_string();

                info!(
                    "SEND MAIL USING: to={}, subject={}, using={} for user={}",
                    to, subject, using_account, user_clone2.user_id
                );

                let state_for_task = Arc::clone(&state_clone2);
                let user_for_task = user_clone2.clone();

                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    let send_err = if let Ok(rt) = rt {
                        let result = rt.block_on(async move {
                            execute_send_mail(
                                &state_for_task,
                                &user_for_task,
                                &to,
                                &subject,
                                &body,
                                vec![],
                                Some(using_account),
                            )
                            .await
                        });
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".to_string()))
                            .err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send SEND MAIL USING result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                    Ok(Ok(message_id)) => Ok(Dynamic::from(message_id)),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("SEND MAIL USING failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "SEND MAIL USING timed out".into(),
                            rhai::Position::NONE,
                        )))
                    }
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("SEND MAIL USING thread failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    let state_clone2 = Arc::clone(&state);
    let user_clone2 = user.clone();

    if let Err(e) = engine
        .register_custom_syntax(
            ["SEND_TEMPLATE", "$expr$", ",", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let recipients_input = context.eval_expression_tree(&inputs[0])?;
                let template = context.eval_expression_tree(&inputs[1])?.to_string();
                let variables = context.eval_expression_tree(&inputs[2])?;

                let mut recipients = Vec::new();
                if recipients_input.is_array() {
                    let arr = recipients_input.cast::<rhai::Array>();
                    for item in arr.iter() {
                        recipients.push(item.to_string());
                    }
                } else {
                    recipients.push(recipients_input.to_string());
                }

                let vars_json = if variables.is_map() {
                    json!(variables.to_string())
                } else {
                    json!({})
                };

                trace!(
                    "SEND_TEMPLATE: recipients={:?}, template={} for user={}",
                    recipients,
                    template,
                    user_clone2.user_id
                );

                let state_for_task = Arc::clone(&state_clone2);
                let user_for_task = user_clone2.clone();

                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    let send_err = if let Ok(rt) = rt {
                        let result = rt.block_on(async move {
                            execute_send_template(
                                &state_for_task,
                                &user_for_task,
                                recipients,
                                &template,
                                vars_json,
                            )
                            .await
                        });
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".to_string()))
                            .err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send SEND_TEMPLATE result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(60)) {
                    Ok(Ok(count)) => Ok(Dynamic::from(count)),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("SEND_TEMPLATE failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                    Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "SEND_TEMPLATE timed out".into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    // Register send_mail as a regular function (for function call style: send_mail(to, subject, body, []))
    let state_fn = Arc::clone(&state);
    let user_fn = user.clone();
    engine.register_fn("send_mail", move |to: Dynamic, subject: Dynamic, body: Dynamic, attachments: Dynamic| -> String {
        // Convert parameters to strings
        let to_str = to.to_string();
        let subject_str = subject.to_string();
        // Convert body to string
        // Convert body to string
        let body_str = body.to_string();

        // Convert attachments to Vec<String>
        let mut atts = Vec::new();
        if attachments.is_array() {
            let arr = attachments.cast::<rhai::Array>(); {
                for item in arr.iter() {
                    atts.push(item.to_string());
                }
            }
        } else if !attachments.to_string().is_empty() {
            atts.push(attachments.to_string());
        }

        // Execute in blocking thread
        let (tx, rx) = std::sync::mpsc::channel();
        let state_for_task = Arc::clone(&state_fn);
        let user_for_task = user_fn.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build();

            let result = if let Ok(rt) = rt {
                match rt.block_on(async {
                    execute_send_mail(&state_for_task, &user_for_task, &to_str, &subject_str, &body_str, atts, None).await
                }) {
                    Ok(msg_id) => Ok(msg_id),
                    Err(e) => {
                        log::error!("execute_send_mail error: {}", e);
                        Err(e)
                    }
                }
            } else {
                Err("Failed to build tokio runtime".to_string())
            };

            let _ = tx.send(result);
        });

        match rx.recv_timeout(std::time::Duration::from_secs(30)) {
            Ok(Ok(message_id)) => message_id,
            Ok(Err(e)) => {
                log::error!("send_mail failed: {}", e);
                String::new()
            },
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                log::error!("send_mail timed out");
                String::new()
            },
            Err(e) => {
                log::error!("send_mail thread failed: {}", e);
                String::new()
            },
        }
    });
}
