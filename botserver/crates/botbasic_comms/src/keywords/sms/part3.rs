use super::*;

pub fn register_send_sms_full_keyword(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();

    if let Err(e) = engine
        .register_custom_syntax(
            ["SEND_SMS", "$expr$", ",", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let phone = context.eval_expression_tree(&inputs[0])?.to_string();
                let message = context.eval_expression_tree(&inputs[1])?.to_string();
                let provider = context.eval_expression_tree(&inputs[2])?.to_string();
                let priority = context.eval_expression_tree(&inputs[3])?.to_string();

                trace!(
                    "SEND_SMS: Sending SMS to {} via {} with priority {}",
                    phone,
                    provider,
                    priority
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
                                Some(&provider),
                                Some(&priority),
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

    let state_clone2 = Arc::clone(&state);
    let user_clone2 = user;

    if let Err(e) = engine
        .register_custom_syntax(
            [
                "SEND_SMS", "$expr$", ",", "$expr$", ",", "$expr$", ",", "$expr$",
            ],
            false,
            move |context, inputs| {
                let phone = context.eval_expression_tree(&inputs[0])?.to_string();
                let message = context.eval_expression_tree(&inputs[1])?.to_string();
                let provider = context.eval_expression_tree(&inputs[2])?.to_string();
                let priority = context.eval_expression_tree(&inputs[3])?.to_string();

                trace!(
                    "SEND_SMS: Sending SMS to {} via {} with priority {}",
                    phone,
                    provider,
                    priority
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
                            execute_send_sms(
                                &*state_for_task,
                                &user_for_task,
                                &phone,
                                &message,
                                Some(&provider),
                                Some(&priority),
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
pub(super) async fn execute_send_sms(
    state: &dyn BasicRuntime,
    user: &UserSession,
    phone: &str,
    message: &str,
    provider_override: Option<&str>,
    priority_override: Option<&str>,
) -> Result<SmsSendResult, Box<dyn std::error::Error + Send + Sync>> {
    let config_manager = ConfigManager::new(state.db_pool().clone());
    let bot_id = user.bot_id;

    let provider_name = match provider_override {
        Some(p) => p.to_string(),
        None => config_manager
            .get_config(&bot_id, "sms-provider", None)
            .unwrap_or_else(|_| "twilio".to_string()),
    };

    let provider = SmsProvider::from(provider_name.as_str());

    let priority = match priority_override {
        Some(p) => SmsPriority::from(p),
        None => {
            let priority_str = config_manager
                .get_config(&bot_id, "sms-default-priority", None)
                .unwrap_or_else(|_| "normal".to_string());
            SmsPriority::from(priority_str.as_str())
        }
    };

    let normalized_phone = normalize_phone_number(phone);

    if matches!(priority, SmsPriority::High | SmsPriority::Urgent) {
        info!(
            "High priority SMS to {}: priority={}",
            normalized_phone, priority
        );
    }

    let result = match provider {
        SmsProvider::Twilio => {
            send_via_twilio(state, &bot_id, &normalized_phone, message, &priority).await
        }
        SmsProvider::AwsSns => {
            send_via_aws_sns(state, &bot_id, &normalized_phone, message, &priority).await
        }
        SmsProvider::Vonage => {
            send_via_vonage(state, &bot_id, &normalized_phone, message, &priority).await
        }
        SmsProvider::MessageBird => {
            send_via_messagebird(state, &bot_id, &normalized_phone, message, &priority).await
        }
        SmsProvider::Custom(name) => {
            send_via_custom_webhook(state, &bot_id, &name, &normalized_phone, message, &priority)
                .await
        }
    };

    match result {
        Ok(message_id) => {
            info!(
                "SMS sent successfully to {} via {} (priority={}): {}",
                normalized_phone,
                provider_name,
                priority,
                message_id.as_deref().unwrap_or("no-id")
            );
            Ok(SmsSendResult {
                success: true,
                message_id,
                provider: provider_name,
                to: normalized_phone,
                priority: priority.to_string(),
                error: None,
            })
        }
        Err(e) => {
            log::error!("SMS send failed to {}: {}", normalized_phone, e);
            Ok(SmsSendResult {
                success: false,
                message_id: None,
                provider: provider_name,
                to: normalized_phone,
                priority: priority.to_string(),
                error: Some(e.to_string()),
            })
        }
    }
}
pub(super) fn normalize_phone_number(phone: &str) -> String {
    let has_plus = phone.starts_with('+');
    let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();

    if has_plus {
        format!("+{}", digits)
    } else if digits.len() == 10 {
        format!("+1{}", digits)
    } else {
        // Both 11-digit starting with '1' and other cases get the same format
        format!("+{}", digits)
    }
}
