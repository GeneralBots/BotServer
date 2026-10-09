use super::*;

pub fn register_copy_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine
        .register_custom_syntax(
            ["COPY", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let source = context.eval_expression_tree(&inputs[0])?.to_string();
                let destination = context.eval_expression_tree(&inputs[1])?.to_string();

                trace!("COPY from {source} to {destination}");

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
                            execute_copy(&state_for_task, &user_for_task, &source, &destination)
                                .await
                        });
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".into())).err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send COPY result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(60)) {
                    Ok(Ok(_)) => Ok(Dynamic::UNIT),
                    Ok(Err(e)) => trap_or_raise(&e.to_string(), "COPY failed"),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "COPY timed out".into(),
                            rhai::Position::NONE,
                        )))
                    }
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("COPY thread failed: {e}").into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub fn register_move_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine
        .register_custom_syntax(
            ["MOVE", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let source = context.eval_expression_tree(&inputs[0])?.to_string();
                let destination = context.eval_expression_tree(&inputs[1])?.to_string();

                trace!("MOVE from {source} to {destination}");

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
                            execute_move(&state_for_task, &user_for_task, &source, &destination)
                                .await
                        });
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".into())).err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send MOVE result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(60)) {
                    Ok(Ok(_)) => Ok(Dynamic::UNIT),
                    Ok(Err(e)) => trap_or_raise(&e.to_string(), "MOVE failed"),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "MOVE timed out".into(),
                            rhai::Position::NONE,
                        )))
                    }
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("MOVE thread failed: {e}").into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub fn register_list_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine
        .register_custom_syntax(["LIST", "$expr$"], false, move |context, inputs| {
            let path = context.eval_expression_tree(&inputs[0])?.to_string();

            trace!("LIST directory: {path}");

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
                        execute_list(&state_for_task, &user_for_task, &path).await
                    });
                    tx.send(result).err()
                } else {
                    tx.send(Err("Failed to build tokio runtime".into())).err()
                };

                if send_err.is_some() {
                    log::error!("Failed to send LIST result from thread");
                }
            });

            match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                Ok(Ok(files)) => {
                    let array: rhai::Array = files.iter().map(|f| Dynamic::from(f.clone())).collect();
                    Ok(Dynamic::from(array))
                }
                Ok(Err(e)) => trap_or_raise(&e.to_string(), "LIST failed"),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "LIST timed out".into(),
                        rhai::Position::NONE,
                    )))
                }
                Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("LIST thread failed: {e}").into(),
                    rhai::Position::NONE,
                ))),
            }
        })
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub fn register_compress_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine
        .register_custom_syntax(
            ["COMPRESS", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let files = context.eval_expression_tree(&inputs[0])?;
                let archive_name = context.eval_expression_tree(&inputs[1])?.to_string();

                trace!("COMPRESS to: {archive_name}");

                let state_for_task = Arc::clone(&state_clone);
                let user_for_task = user_clone.clone();

                let file_list: Vec<String> = if files.is_array() {
                    files
                        .into_array()
                        .unwrap_or_default()
                        .iter()
                        .map(|f| f.to_string())
                        .collect()
                } else {
                    vec![files.to_string()]
                };

                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    let send_err = if let Ok(rt) = rt {
                        let result = rt.block_on(async move {
                            execute_compress(
                                &state_for_task,
                                &user_for_task,
                                &file_list,
                                &archive_name,
                            )
                            .await
                        });
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".into())).err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send COMPRESS result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(120)) {
                    Ok(Ok(path)) => Ok(Dynamic::from(path)),
                    Ok(Err(e)) => trap_or_raise(&e.to_string(), "COMPRESS failed"),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "COMPRESS timed out".into(),
                            rhai::Position::NONE,
                        )))
                    }
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("COMPRESS thread failed: {e}").into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
