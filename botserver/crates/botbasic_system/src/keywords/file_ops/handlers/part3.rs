use super::*;

pub fn register_extract_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine
        .register_custom_syntax(
            ["EXTRACT", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let archive = context.eval_expression_tree(&inputs[0])?.to_string();
                let destination = context.eval_expression_tree(&inputs[1])?.to_string();

                trace!("EXTRACT {archive} to {destination}");

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
                            execute_extract(&state_for_task, &user_for_task, &archive, &destination)
                                .await
                        });
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".into())).err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send EXTRACT result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(120)) {
                    Ok(Ok(files)) => {
                        let array: rhai::Array = files.iter().map(|f| Dynamic::from(f.clone())).collect();
                        Ok(Dynamic::from(array))
                    }
                    Ok(Err(e)) => trap_or_raise(&e.to_string(), "EXTRACT failed"),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "EXTRACT timed out".into(),
                            rhai::Position::NONE,
                        )))
                    }
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("EXTRACT thread failed: {e}").into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub fn register_upload_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine
        .register_custom_syntax(
            ["UPLOAD", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let file = context.eval_expression_tree(&inputs[0])?;
                let destination = context.eval_expression_tree(&inputs[1])?.to_string();

                trace!("UPLOAD to: {destination}");

                let state_for_task = Arc::clone(&state_clone);
                let user_for_task = user_clone.clone();
                let file_data = dynamic_to_file_data(&file);

                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    let send_err = if let Ok(rt) = rt {
                        let result = rt.block_on(async move {
                            execute_upload(&state_for_task, &user_for_task, file_data, &destination)
                                .await
                        });
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".into())).err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send UPLOAD result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(300)) {
                    Ok(Ok(url)) => Ok(Dynamic::from(url)),
                    Ok(Err(e)) => trap_or_raise(&e.to_string(), "UPLOAD failed"),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "UPLOAD timed out".into(),
                            rhai::Position::NONE,
                        )))
                    }
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("UPLOAD thread failed: {e}").into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub fn register_download_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine
        .register_custom_syntax(
            ["DOWNLOAD", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let url = context.eval_expression_tree(&inputs[0])?.to_string();
                let local_path = context.eval_expression_tree(&inputs[1])?.to_string();

                trace!("DOWNLOAD {url} to {local_path}");

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
                            execute_download(&state_for_task, &user_for_task, &url, &local_path)
                                .await
                        });
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".into())).err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send DOWNLOAD result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(300)) {
                    Ok(Ok(path)) => Ok(Dynamic::from(path)),
                    Ok(Err(e)) => trap_or_raise(&e.to_string(), "DOWNLOAD failed"),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "DOWNLOAD timed out".into(),
                            rhai::Position::NONE,
                        )))
                    }
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("DOWNLOAD thread failed: {e}").into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
