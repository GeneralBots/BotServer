use super::*;

pub fn register_delete_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();
    let user_clone = user.clone();
    let user_roles = UserRoles::from_user_session(&user);

    if let Err(e) = engine
        .register_custom_syntax(
            ["DELETE", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let first_arg = context.eval_expression_tree(&inputs[0])?.to_string();
                let second_arg = context.eval_expression_tree(&inputs[1])?.to_string();

                if first_arg.starts_with("http://") || first_arg.starts_with("https://") {
                    trace!("DELETE HTTP with data: {}", first_arg);

                    let (tx, rx) = std::sync::mpsc::channel();
                    let url_clone = first_arg;

                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Builder::new_current_thread()
                            .worker_threads(2)
                            .enable_all()
                            .build();

                        let _ = if let Ok(rt) = rt {
                            let result = rt.block_on(async move {
                                let client = reqwest::Client::new();
                                client
                                    .delete(&url_clone)
                                    .timeout(std::time::Duration::from_secs(60))
                                    .send()
                                    .await
                                    .map_err(|e| format!("HTTP error: {}", e))?
                                    .text()
                                    .await
                                    .map_err(|e| format!("Response error: {}", e))
                            });
                            tx.send(result)
                        } else {
                            tx.send(Err("Failed to build runtime".to_string()))
                        };
                    });

                    match rx.recv_timeout(std::time::Duration::from_secs(60)) {
                        Ok(Ok(response)) => Ok(Dynamic::from(response)),
                        Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            format!("DELETE failed: {}", e).into(),
                            rhai::Position::NONE,
                        ))),
                        Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "DELETE timed out".into(),
                            rhai::Position::NONE,
                        ))),
                    }
                } else {
                    trace!("DELETE from table: {}, filter: {}", first_arg, second_arg);

                    let pool_opt = state_clone
                        .bot_database_manager()
                        .get_bot_pool(user_clone.bot_id);
                    let mut conn = if let Some(pool) = pool_opt {
                        pool.get().map_err(|e| format!("Bot DB error: {}", e))?
                    } else {
                        state_clone
                            .db_pool()
                            .get()
                            .map_err(|e| format!("DB error: {}", e))?
                    };

                    // Check write access (delete requires write permission)
                    if let Err(e) =
                        check_table_access(&mut conn, &first_arg, &user_roles, AccessType::Write)
                    {
                        warn!("DELETE access denied: {}", e);
                        return Err(e.into());
                    }

                    let result = execute_delete(&mut conn, &first_arg, &second_arg)
                        .map_err(|e| format!("DELETE error: {}", e))?;

                    Ok(Dynamic::from(result))
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    let state_clone2 = state.clone();
    if let Err(e) = engine
        .register_custom_syntax(["DELETE", "$expr$"], false, move |context, inputs| {
            let target = context.eval_expression_tree(&inputs[0])?.to_string();

            if target.starts_with("http://") || target.starts_with("https://") {
                trace!("DELETE HTTP: {}", target);

                let (tx, rx) = std::sync::mpsc::channel();
                let url_clone = target;

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    let _ = if let Ok(rt) = rt {
                        let result = rt.block_on(async move {
                            let client = reqwest::Client::new();
                            client
                                .delete(&url_clone)
                                .timeout(std::time::Duration::from_secs(60))
                                .send()
                                .await
                                .map_err(|e| format!("HTTP error: {}", e))?
                                .text()
                                .await
                                .map_err(|e| format!("Response error: {}", e))
                        });
                        tx.send(result)
                    } else {
                        tx.send(Err("Failed to build runtime".to_string()))
                    };
                });

                match rx.recv_timeout(std::time::Duration::from_secs(60)) {
                    Ok(Ok(response)) => Ok(Dynamic::from(response)),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("DELETE failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                    Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "DELETE timed out".into(),
                        rhai::Position::NONE,
                    ))),
                }
            } else {
                trace!("DELETE file: {}", target);

                let _state = Arc::clone(&state_clone2);

                let file_path = std::path::Path::new(&target);
                if file_path.exists() {
                    std::fs::remove_file(file_path)
                        .map_err(|e| format!("File delete error: {}", e))?;
                    Ok(Dynamic::from(true))
                } else {
                    Ok(Dynamic::from(format!("File not found: {}", target)))
                }
            }
        })
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub fn register_merge_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();
    let user_clone = user.clone();

    if let Err(e) = engine
        .register_custom_syntax(
            ["MERGE", "$expr$", ",", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let table = context.eval_expression_tree(&inputs[0])?.to_string();
                let data = context.eval_expression_tree(&inputs[1])?;
                let key_field = context.eval_expression_tree(&inputs[2])?.to_string();

                trace!("MERGE into table: {}, key: {}", table, key_field);

                let pool_opt = state_clone
                    .bot_database_manager()
                    .get_bot_pool(user_clone.bot_id);
                let mut conn = if let Some(pool) = pool_opt {
                    pool.get().map_err(|e| format!("Bot DB error: {}", e))?
                } else {
                    state_clone
                        .db_pool()
                        .get()
                        .map_err(|e| format!("DB error: {}", e))?
                };

                let result = execute_merge(&mut conn, &table, &data, &key_field)
                    .map_err(|e| format!("MERGE error: {}", e))?;

                Ok(json_value_to_dynamic(&result))
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub fn register_fill_keyword(_state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine
        .register_custom_syntax(
            ["FILL", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let data = context.eval_expression_tree(&inputs[0])?;
                let template = context.eval_expression_tree(&inputs[1])?;

                trace!("FILL with template");

                let result = execute_fill(&data, &template)?;

                Ok(result)
            },
        )
{
    log::error!("Failed to register the custom syntax: {e}");
}
}
pub fn register_map_keyword(_state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine
        .register_custom_syntax(
            ["MAP", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let data = context.eval_expression_tree(&inputs[0])?;
                let mapping = context.eval_expression_tree(&inputs[1])?.to_string();

                trace!("MAP with mapping: {}", mapping);

                let result = execute_map(&data, &mapping)?;

                Ok(result)
            },
        )
{
    log::error!("Failed to register the custom syntax: {e}");
}
}
pub fn register_filter_keyword(_state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine
        .register_custom_syntax(
            ["FILTER", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let data = context.eval_expression_tree(&inputs[0])?;
                let condition = context.eval_expression_tree(&inputs[1])?.to_string();

                trace!("FILTER with condition: {}", condition);

                let result = execute_filter(&data, &condition)?;

                Ok(result)
            },
        )
{
    log::error!("Failed to register the custom syntax: {e}");
}
}
pub fn register_aggregate_keyword(_state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine
        .register_custom_syntax(
            ["AGGREGATE", "$expr$", ",", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let operation = context.eval_expression_tree(&inputs[0])?.to_string();
                let data = context.eval_expression_tree(&inputs[1])?;
                let field = context.eval_expression_tree(&inputs[2])?.to_string();

                trace!("AGGREGATE {} on field: {}", operation, field);

                let result = execute_aggregate(&operation, &data, &field)?;

                Ok(result)
            },
        )
{
    log::error!("Failed to register the custom syntax: {e}");
}
}
pub fn register_join_keyword(_state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine
        .register_custom_syntax(
            ["JOIN", "$expr$", ",", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let left = context.eval_expression_tree(&inputs[0])?;
                let right = context.eval_expression_tree(&inputs[1])?;
                let key = context.eval_expression_tree(&inputs[2])?.to_string();

                trace!("JOIN on key: {}", key);

                let result = execute_join(&left, &right, &key)?;

                Ok(result)
            },
        )
{
    log::error!("Failed to register the custom syntax: {e}");
}
}
