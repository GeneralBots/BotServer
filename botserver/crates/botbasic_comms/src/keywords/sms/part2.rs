use super::*;

pub fn register_sms_keywords(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    register_send_sms_keyword(Arc::clone(&state), user.clone(), engine);
    register_send_sms_with_third_arg_keyword(Arc::clone(&state), user.clone(), engine);
    register_send_sms_full_keyword(state, user, engine);
}
pub fn register_send_sms_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine
        .register_custom_syntax(
            ["SEND_SMS", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let phone = context.eval_expression_tree(&inputs[0])?.to_string();
                let message = context.eval_expression_tree(&inputs[1])?.to_string();

                trace!("SEND_SMS: Sending SMS to {}", phone);

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
                            execute_send_sms(
                                &*state_for_task,
                                &user_for_task,
                                &phone,
                                &message,
                                None,
                                None,
                            )
                            .await
                        });
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".into())).err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send SEND_SMS result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                    Ok(Ok(result)) => {
                        let mut map = rhai::Map::new();
                        map.insert("success".into(), Dynamic::from(result.success));
                        map.insert(
                            "message_id".into(),
                            Dynamic::from(result.message_id.unwrap_or_default()),
                        );
                        map.insert("provider".into(), Dynamic::from(result.provider));
                        map.insert("to".into(), Dynamic::from(result.to));
                        map.insert("priority".into(), Dynamic::from(result.priority));
                        if let Some(err) = result.error {
                            map.insert("error".into(), Dynamic::from(err));
                        }
                        Ok(Dynamic::from(map))
                    }
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("SEND_SMS failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "SEND_SMS timed out".into(),
                            rhai::Position::NONE,
                        )))
                    }
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("SEND_SMS thread failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub fn register_send_sms_with_third_arg_keyword(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine
        .register_custom_syntax(
            ["SEND_SMS", "$expr$", ",", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let phone = context.eval_expression_tree(&inputs[0])?.to_string();
                let message = context.eval_expression_tree(&inputs[1])?.to_string();
                let third_arg = context.eval_expression_tree(&inputs[2])?.to_string();

                let is_priority = matches!(
                    third_arg.to_lowercase().as_str(),
                    "low" | "normal" | "high" | "urgent" | "critical"
                );

                let (provider_override, priority_override) = if is_priority {
                    (None, Some(third_arg.clone()))
                } else {
                    (Some(third_arg.clone()), None)
                };

                trace!(
                    "SEND_SMS: Sending SMS to {} (third_arg={}, is_priority={})",
                    phone,
                    third_arg,
                    is_priority
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
                            execute_send_sms(
                                &*state_for_task,
                                &user_for_task,
                                &phone,
                                &message,
                                provider_override.as_deref(),
                                priority_override.as_deref(),
                            )
                            .await
                        });
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".into())).err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send SEND_SMS result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                    Ok(Ok(result)) => {
                        let mut map = rhai::Map::new();
                        map.insert("success".into(), Dynamic::from(result.success));
                        map.insert(
                            "message_id".into(),
                            Dynamic::from(result.message_id.unwrap_or_default()),
                        );
                        map.insert("provider".into(), Dynamic::from(result.provider));
                        map.insert("to".into(), Dynamic::from(result.to));
                        map.insert("priority".into(), Dynamic::from(result.priority));
                        if let Some(err) = result.error {
                            map.insert("error".into(), Dynamic::from(err));
                        }
                        Ok(Dynamic::from(map))
                    }
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("SEND_SMS failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "SEND_SMS timed out".into(),
                            rhai::Position::NONE,
                        )))
                    }
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("SEND_SMS thread failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
