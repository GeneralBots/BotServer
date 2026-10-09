use super::*;

/// A BASIC-level operation error from a file keyword: honor
/// `ON ERROR RESUME NEXT` (record the last error, continue) instead of
/// raising a raw Rhai error the trap cannot catch. classify_media.bas relies
/// on this to exit quietly when the model's tool call races the
/// media_uploaded event trigger over the same upload.
pub(super) fn trap_or_raise(op_err: &str, label: &str) -> Result<Dynamic, Box<rhai::EvalAltResult>> {
    let message = format!("{label}: {op_err}");
    if botbasic_core::keywords::errors::is_error_resume_next_active() {
        botbasic_core::keywords::errors::set_last_error(&message, 1);
        log::trace!("File keyword error suppressed by ON ERROR RESUME NEXT: {message}");
        return Ok(Dynamic::UNIT);
    }
    Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
        message.into(),
        rhai::Position::NONE,
    )))
}
pub fn register_file_operations(state: &Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    register_read_keyword(Arc::clone(state), user.clone(), engine);
    register_write_keyword(Arc::clone(state), user.clone(), engine);
    register_create_file_keyword(Arc::clone(state), user.clone(), engine);
    register_create_shortcut_keyword(Arc::clone(state), user.clone(), engine);
    register_delete_file_keyword(Arc::clone(state), user.clone(), engine);
    register_copy_keyword(Arc::clone(state), user.clone(), engine);
    register_move_keyword(Arc::clone(state), user.clone(), engine);
    register_list_keyword(Arc::clone(state), user.clone(), engine);
    register_compress_keyword(Arc::clone(state), user.clone(), engine);
    register_extract_keyword(Arc::clone(state), user.clone(), engine);
    register_upload_keyword(Arc::clone(state), user.clone(), engine);
    register_download_keyword(Arc::clone(state), user.clone(), engine);
    register_generate_pdf_keyword(Arc::clone(state), user.clone(), engine);
    register_merge_pdf_keyword(Arc::clone(state), user, engine);
}
pub fn register_read_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine
        .register_custom_syntax(["READ", "$expr$"], false, move |context, inputs| {
            let path = context.eval_expression_tree(&inputs[0])?.to_string();

            trace!("READ file: {path}");

            let state_for_task = Arc::clone(&state);
            let user_for_task = user.clone();

            let (tx, rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build();

                let send_err = if let Ok(rt) = rt {
                    let result = rt.block_on(async move {
                        execute_read(&state_for_task, &user_for_task, &path).await
                    });
                    tx.send(result).err()
                } else {
                    tx.send(Err("Failed to build tokio runtime".into())).err()
                };

                if send_err.is_some() {
                    log::error!("Failed to send READ result from thread");
                }
            });

            match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                Ok(Ok(content)) => Ok(Dynamic::from(content)),
                Ok(Err(e)) => trap_or_raise(&e.to_string(), "READ failed"),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "READ timed out".into(),
                        rhai::Position::NONE,
                    )))
                }
                Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("READ thread failed: {e}").into(),
                    rhai::Position::NONE,
                ))),
            }
        })
{
    log::error!("Failed to register the custom syntax: {e}");
}
}
pub fn register_write_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine
        .register_custom_syntax(
            ["WRITE", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let path = context.eval_expression_tree(&inputs[0])?.to_string();
                let data = context.eval_expression_tree(&inputs[1])?;

                trace!("WRITE to file: {path}");

                let state_for_task = Arc::clone(&state_clone);
                let user_for_task = user_clone.clone();
                let data_str = if data.is_string() {
                    data.to_string()
                } else {
                    serde_json::to_string(&crate::keywords::file_ops::utils::dynamic_to_json(&data)).unwrap_or_default()
                };

                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    let send_err = if let Ok(rt) = rt {
                        let result = rt.block_on(async move {
                            execute_write(&state_for_task, &user_for_task, &path, &data_str).await
                        });
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".into())).err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send WRITE result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                    Ok(Ok(_)) => Ok(Dynamic::UNIT),
                    Ok(Err(e)) => trap_or_raise(&e.to_string(), "WRITE failed"),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "WRITE timed out".into(),
                            rhai::Position::NONE,
                        )))
                    }
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("WRITE thread failed: {e}").into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub fn register_delete_file_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();
    let state_clone2 = Arc::clone(&state);
    let user_clone2 = user;

    if let Err(e) = engine
        .register_custom_syntax(
            ["DELETE", "FILE", "$expr$"],
            false,
            move |context, inputs| {
                let path = context.eval_expression_tree(&inputs[0])?.to_string();

                trace!("DELETE FILE: {path}");

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
                            execute_delete_file(&state_for_task, &user_for_task, &path).await
                        });
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".into())).err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send DELETE FILE result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                    Ok(Ok(_)) => Ok(Dynamic::UNIT),
                    Ok(Err(e)) => trap_or_raise(&e.to_string(), "DELETE FILE failed"),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "DELETE FILE timed out".into(),
                            rhai::Position::NONE,
                        )))
                    }
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("DELETE FILE thread failed: {e}").into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    if let Err(e) = engine
        .register_custom_syntax(
            ["DELETE", "FILE", "$expr$"],
            false,
            move |context, inputs| {
                let path = context.eval_expression_tree(&inputs[0])?.to_string();

                trace!("DELETE FILE: {path}");

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
                            execute_delete_file(&state_for_task, &user_for_task, &path).await
                        });
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".into())).err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send DELETE FILE result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                    Ok(Ok(_)) => Ok(Dynamic::UNIT),
                    Ok(Err(e)) => trap_or_raise(&e.to_string(), "DELETE FILE failed"),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "DELETE FILE timed out".into(),
                            rhai::Position::NONE,
                        )))
                    }
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("DELETE FILE thread failed: {e}").into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
        {
            log::error!("Failed to register the custom syntax: {e}");
        }
}
