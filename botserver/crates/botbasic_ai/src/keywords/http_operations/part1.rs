use super::*;

pub fn register_http_operations(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    register_post_keyword(Arc::clone(&state), user.clone(), engine);
    register_put_keyword(Arc::clone(&state), user.clone(), engine);
    register_patch_keyword(Arc::clone(&state), user.clone(), engine);
    register_delete_http_keyword(Arc::clone(&state), user.clone(), engine);
    register_set_header_keyword(Arc::clone(&state), user.clone(), engine);
    register_graphql_keyword(Arc::clone(&state), user.clone(), engine);
    register_soap_keyword(Arc::clone(&state), user.clone(), engine);
    register_clear_headers_keyword(state, user, engine);
}
pub fn register_post_keyword(state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["POST", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let url = context.eval_expression_tree(&inputs[0])?.to_string();
                let data = context.eval_expression_tree(&inputs[1])?;

                trace!("POST request to: {url}");

                let (tx, rx) = std::sync::mpsc::channel();
                let url_clone = url;
                let data_clone = dynamic_to_json(&data);

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    let send_err = if let Ok(rt) = rt {
                        let result = rt.block_on(async move {
                            execute_http_request(Method::POST, &url_clone, Some(data_clone), None)
                                .await
                        });
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".into())).err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send POST result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(60)) {
                    Ok(Ok(response)) => Ok(json_to_dynamic(&response)),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("POST failed: {e}").into(),
                        rhai::Position::NONE,
                    ))),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "POST request timed out".into(),
                            rhai::Position::NONE,
                        )))
                    }
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("POST thread failed: {e}").into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub fn register_put_keyword(state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["PUT", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let url = context.eval_expression_tree(&inputs[0])?.to_string();
                let data = context.eval_expression_tree(&inputs[1])?;

                trace!("PUT request to: {url}");

                let (tx, rx) = std::sync::mpsc::channel();
                let url_clone = url;
                let data_clone = dynamic_to_json(&data);

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    let send_err = if let Ok(rt) = rt {
                        let result = rt.block_on(async move {
                            execute_http_request(Method::PUT, &url_clone, Some(data_clone), None)
                                .await
                        });
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".into())).err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send PUT result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(60)) {
                    Ok(Ok(response)) => Ok(json_to_dynamic(&response)),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("PUT failed: {e}").into(),
                        rhai::Position::NONE,
                    ))),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "PUT request timed out".into(),
                            rhai::Position::NONE,
                        )))
                    }
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("PUT thread failed: {e}").into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub fn register_patch_keyword(state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["PATCH", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let url = context.eval_expression_tree(&inputs[0])?.to_string();
                let data = context.eval_expression_tree(&inputs[1])?;

                trace!("PATCH request to: {url}");

                let (tx, rx) = std::sync::mpsc::channel();
                let url_clone = url;
                let data_clone = dynamic_to_json(&data);

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    let send_err = if let Ok(rt) = rt {
                        let result = rt.block_on(async move {
                            execute_http_request(Method::PATCH, &url_clone, Some(data_clone), None)
                                .await
                        });
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".into())).err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send PATCH result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(60)) {
                    Ok(Ok(response)) => Ok(json_to_dynamic(&response)),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("PATCH failed: {e}").into(),
                        rhai::Position::NONE,
                    ))),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "PATCH request timed out".into(),
                            rhai::Position::NONE,
                        )))
                    }
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("PATCH thread failed: {e}").into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub fn register_delete_http_keyword(
    _state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    if let Err(e) = engine
        .register_custom_syntax(
            ["DELETE", "HTTP", "$expr$"],
            false,
            move |context, inputs| {
                let url = context.eval_expression_tree(&inputs[0])?.to_string();

                trace!("DELETE HTTP request to: {url}");

                let (tx, rx) = std::sync::mpsc::channel();
                let url_clone = url;

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    let send_err = if let Ok(rt) = rt {
                        let result = rt.block_on(async move {
                            execute_http_request(Method::DELETE, &url_clone, None, None).await
                        });
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".into())).err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send DELETE result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(60)) {
                    Ok(Ok(response)) => Ok(json_to_dynamic(&response)),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("DELETE failed: {e}").into(),
                        rhai::Position::NONE,
                    ))),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "DELETE request timed out".into(),
                            rhai::Position::NONE,
                        )))
                    }
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("DELETE thread failed: {e}").into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
{
    log::error!("Failed to register the custom syntax: {e}");
}
}
