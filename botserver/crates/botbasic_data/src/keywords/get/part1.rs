use super::*;

pub fn register_get_keyword(state: Arc<dyn BasicRuntime>, user_session: UserSession, engine: &mut Engine) {
    register_get_file_fn(state.clone(), user_session.clone(), engine);
    register_get_syntax(state, user_session, engine);
}
/// Function form `get_file(path)`, emitted by `convert_multiword_keywords`.
/// The `GET $expr$` custom syntax is overridden at runtime by the later
/// GET SHAREPOINT/INSTAGRAM/… registrations (same Rhai first-token key).
pub(super) fn register_get_file_fn(state: Arc<dyn BasicRuntime>, user_session: UserSession, engine: &mut Engine) {
    let state_fn = Arc::clone(&state);
    let user_fn = user_session;
    engine.register_fn("get_file", move |file_path: String| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
        let state_for_blocking = Arc::clone(&state_fn);
        let path_str = file_path;
        let bot_id = user_fn.bot_id;
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .worker_threads(2)
                .enable_all()
                .build();
            let send_err = if let Ok(rt) = rt {
                let result = rt.block_on(async move {
                    if path_str.starts_with("https://") || path_str.starts_with("http://") {
                        execute_get(&path_str).await
                    } else {
                        get_from_bucket(state_for_blocking, &path_str, bot_id).await
                    }
                });
                tx.send(result).err()
            } else {
                tx.send(Err("failed to build tokio runtime".into())).err()
            };
            if send_err.is_some() {
                log::error!("Failed to send result from thread");
            }
        });
        match rx.recv_timeout(std::time::Duration::from_secs(40)) {
            Ok(Ok(content)) => {
                botbasic_core::keywords::errors::clear_last_error();
                Ok(Dynamic::from(content))
            }
            Ok(Err(e)) => {
                botbasic_core::keywords::errors::set_last_error(&e.to_string(), 1);
                if botbasic_core::keywords::errors::is_error_resume_next_active() {
                    Ok(Dynamic::from(String::new()))
                } else {
                    Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        e.to_string().into(),
                        rhai::Position::NONE,
                    )))
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                botbasic_core::keywords::errors::set_last_error("GET timed out", 1);
                if botbasic_core::keywords::errors::is_error_resume_next_active() {
                    Ok(Dynamic::from(String::new()))
                } else {
                    Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "GET timed out".into(),
                        rhai::Position::NONE,
                    )))
                }
            }
            Err(e) => {
                botbasic_core::keywords::errors::set_last_error(&format!("GET failed: {e}"), 1);
                if botbasic_core::keywords::errors::is_error_resume_next_active() {
                    Ok(Dynamic::from(String::new()))
                } else {
                    Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("GET failed: {e}").into(),
                        rhai::Position::NONE,
                    )))
                }
            }
        }
    });
}
pub(super) fn register_get_syntax(state: Arc<dyn BasicRuntime>, user_session: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    if let Err(e) = engine
        .register_custom_syntax(["GET", "$expr$"], false, move |context, inputs| {
            let url = context.eval_expression_tree(&inputs[0])?;
            let url_str = url.to_string();
            if !is_safe_path(&url_str) {
                return Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    "URL contains invalid or unsafe path sequences".into(),
                    rhai::Position::NONE,
                )));
            }
            let state_for_blocking = Arc::clone(&state_clone);
            let url_for_blocking = url_str;
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build();
                let send_err = if let Ok(rt) = rt {
                    let result = rt.block_on(async move {
                        if url_for_blocking.starts_with("https://")
                            || url_for_blocking.starts_with("http://")
                        {
                            execute_get(&url_for_blocking).await
                        } else {
                            get_from_bucket(
                                state_for_blocking,
                                &url_for_blocking,
                                user_session.bot_id,
                            )
                            .await
                        }
                    });
                    tx.send(result).err()
                } else {
                    tx.send(Err("failed to build tokio runtime".into())).err()
                };
                if send_err.is_some() {
                    log::error!("Failed to send result from thread");
                }
            });
            match rx.recv_timeout(std::time::Duration::from_secs(40)) {
                Ok(Ok(content)) => {
                    // Success resets the `IF ERROR THEN` flag (thread-local
                    // state would otherwise leak between keyword calls).
                    botbasic_core::keywords::errors::clear_last_error();
                    Ok(Dynamic::from(content))
                }
                Ok(Err(e)) => {
                    // BASIC `ON ERROR RESUME NEXT` contract: when the flag is
                    // active, record for `IF ERROR THEN` and yield UNIT so the
                    // script continues with an empty value; else propagate.
                    botbasic_core::keywords::errors::set_last_error(&e.to_string(), 1);
                    if botbasic_core::keywords::errors::is_error_resume_next_active() {
                        Ok(Dynamic::from(String::new()))
                    } else {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            e.to_string().into(),
                            rhai::Position::NONE,
                        )))
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    botbasic_core::keywords::errors::set_last_error("GET timed out", 1);
                    if botbasic_core::keywords::errors::is_error_resume_next_active() {
                        Ok(Dynamic::from(String::new()))
                    } else {
                        Err(Box::new(
                            rhai::EvalAltResult::ErrorRuntime("GET timed out".into(), rhai::Position::NONE),
                        ))
                    }
                }
                Err(e) => {
                    botbasic_core::keywords::errors::set_last_error(&format!("GET failed: {e}"), 1);
                    if botbasic_core::keywords::errors::is_error_resume_next_active() {
                        Ok(Dynamic::from(String::new()))
                    } else {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            format!("GET failed: {e}").into(),
                            rhai::Position::NONE,
                        )))
                    }
                }
            }
        })
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub fn register_get_from_fn(
    state: Arc<dyn BasicRuntime>,
    user_session: UserSession,
    engine: &mut Engine,
) {
    let state_fn = Arc::clone(&state);
    let user_fn = user_session.clone();
    engine.register_fn("GET_FROM", move |table: String| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
        let filter = "1=1".to_string();
        let bot_id = user_fn.bot_id;
        let pool = state_fn.db_pool().clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let tbl = table;
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            let result = match rt {
                Ok(rt) => rt.block_on(async move {
                    execute_get_from(&pool, &tbl, &filter, bot_id)
                }),
                Err(e) => Err(format!("Failed to create runtime: {e}")),
            };
            let _ = tx.send(result);
        });
        let result = rx.recv().unwrap_or(Err("Failed to receive result".into()))?;
        let results = result.get("results").and_then(|r| r.as_array()).cloned().unwrap_or_default();
        let results_value = serde_json::Value::Array(results);
        Ok(botbasic_core::utils::json_value_to_dynamic(&results_value))
    });

    let state_fn2 = Arc::clone(&state);
    let user_fn2 = user_session;
    engine.register_fn("GET_FROM", move |table: String, filter: String| -> Result<Dynamic, Box<rhai::EvalAltResult>> {
        let bot_id = user_fn2.bot_id;
        let pool = state_fn2.db_pool().clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let tbl = table;
        let flt = filter;
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            let result = match rt {
                Ok(rt) => rt.block_on(async move {
                    execute_get_from(&pool, &tbl, &flt, bot_id)
                }),
                Err(e) => Err(format!("Failed to create runtime: {e}")),
            };
            let _ = tx.send(result);
        });
        let result = rx.recv().unwrap_or(Err("Failed to receive result".into()))?;
        let results = result.get("results").and_then(|r| r.as_array()).cloned().unwrap_or_default();
        let results_value = serde_json::Value::Array(results);
        Ok(botbasic_core::utils::json_value_to_dynamic(&results_value))
    });
}
pub fn register_get_from_keyword(
    state: Arc<dyn BasicRuntime>,
    user_session: UserSession,
    engine: &mut Engine,
) {
    let state_for_closure = state.clone();
    let user_for_closure = user_session;
    if let Err(e) = engine
        .register_custom_syntax(
            ["GET", "FROM", "$expr$", "WHERE", "$expr$"],
            false,
            move |context, inputs| {
                let table = context.eval_expression_tree(&inputs[0])?.to_string();
                let filter = context.eval_expression_tree(&inputs[1])?.to_string();
                let bot_id = user_for_closure.bot_id;

                let pool = state_for_closure.db_pool().clone();
                let (tx, rx) = std::sync::mpsc::channel();
                let table_clone = table.clone();
                let filter_clone = filter.clone();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build();
                    let result = match rt {
                        Ok(rt) => rt.block_on(async move {
                            execute_get_from(&pool, &table_clone, &filter_clone, bot_id)
                        }),
                        Err(e) => Err(format!("Failed to create runtime: {e}")),
                    };
                    let _ = tx.send(result);
                });
                let result = rx.recv().unwrap_or(Err("Failed to receive result".into()))?;

                let results = result.get("results").and_then(|r| r.as_array()).cloned().unwrap_or_default();
                let first = results.into_iter().next().unwrap_or(Value::Null);
                let dynamic = botbasic_core::utils::json_value_to_dynamic(&first);
                Ok(dynamic)
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
