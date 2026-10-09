use super::*;

pub fn run_python_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine
        .register_custom_syntax(
            ["RUN", "PYTHON", "$expr$"],
            false,
            move |context, inputs| {
                let code = context
                    .eval_expression_tree(&inputs[0])?
                    .to_string()
                    .trim_matches('"')
                    .to_string();

                trace!("RUN PYTHON for session: {}", user_clone.id);

                let state_for_task = Arc::clone(&state_clone);
                let session_id = user_clone.id;
                let bot_id = user_clone.bot_id;

                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let rt = match tokio::runtime::Runtime::new() {
                        Ok(rt) => rt,
                        Err(e) => {
                            let _ = tx.send(ExecutionResult::error(&format!("Failed to create runtime: {}", e)));
                            return;
                        }
                    };
                    let result = rt.block_on(async {
                        let config = SandboxConfig::from_bot_config(&*state_for_task, bot_id);
                        let sandbox = CodeSandbox::new(config, session_id);
                        sandbox.execute(&code, CodeLanguage::Python).await
                    });
                    let _ = tx.send(result);
                });

                match rx.recv_timeout(std::time::Duration::from_secs(60)) {
                    Ok(result) => Ok(Dynamic::from(result.output())),
                    Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "RUN PYTHON timed out".into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register RUN PYTHON syntax failed: {e}");
    }
}
pub fn run_javascript_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();

    if let Err(e) = engine
        .register_custom_syntax(
            ["RUN", "JAVASCRIPT", "$expr$"],
            false,
            move |context, inputs| {
                let code = context
                    .eval_expression_tree(&inputs[0])?
                    .to_string()
                    .trim_matches('"')
                    .to_string();

                trace!("RUN JAVASCRIPT for session: {}", user_clone.id);

                let state_for_task = Arc::clone(&state_clone);
                let session_id = user_clone.id;
                let bot_id = user_clone.bot_id;

                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let rt = match tokio::runtime::Runtime::new() {
                        Ok(rt) => rt,
                        Err(e) => {
                            let _ = tx.send(ExecutionResult::error(&format!("Failed to create runtime: {}", e)));
                            return;
                        }
                    };
                    let result = rt.block_on(async {
                        let config = SandboxConfig::from_bot_config(&*state_for_task, bot_id);
                        let sandbox = CodeSandbox::new(config, session_id);
                        sandbox.execute(&code, CodeLanguage::JavaScript).await
                    });
                    let _ = tx.send(result);
                });

                match rx.recv_timeout(std::time::Duration::from_secs(60)) {
                    Ok(result) => Ok(Dynamic::from(result.output())),
                    Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "RUN JAVASCRIPT timed out".into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register RUN JAVASCRIPT syntax failed: {e}");
    }

    if let Err(e) = engine
        .register_custom_syntax(["RUN", "JS", "$expr$"], false, move |context, inputs| {
            let code = context
                .eval_expression_tree(&inputs[0])?
                .to_string()
                .trim_matches('"')
                .to_string();

            let state_for_task = Arc::clone(&state);
            let session_id = user.id;
            let bot_id = user.bot_id;

            let (tx, rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        let _ = tx.send(ExecutionResult::error(&format!("Failed to create runtime: {}", e)));
                        return;
                    }
                };
                let result = rt.block_on(async {
                    let config = SandboxConfig::from_bot_config(&*state_for_task, bot_id);
                    let sandbox = CodeSandbox::new(config, session_id);
                    sandbox.execute(&code, CodeLanguage::JavaScript).await
                });
                let _ = tx.send(result);
            });

            match rx.recv_timeout(std::time::Duration::from_secs(60)) {
                Ok(result) => Ok(Dynamic::from(result.output())),
                Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    "RUN JS timed out".into(),
                    rhai::Position::NONE,
                ))),
            }
        })
    {
        log::error!("Failed to register RUN JS syntax failed: {e}");
    }
}
pub fn run_bash_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine
        .register_custom_syntax(["RUN", "BASH", "$expr$"], false, move |context, inputs| {
            let code = context
                .eval_expression_tree(&inputs[0])?
                .to_string()
                .trim_matches('"')
                .to_string();

            trace!("RUN BASH for session: {}", user.id);

            let state_for_task = Arc::clone(&state);
            let session_id = user.id;
            let bot_id = user.bot_id;

            let (tx, rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        let _ = tx.send(ExecutionResult::error(&format!("Failed to create runtime: {}", e)));
                        return;
                    }
                };
                let result = rt.block_on(async {
                    let config = SandboxConfig::from_bot_config(&*state_for_task, bot_id);
                    let sandbox = CodeSandbox::new(config, session_id);
                    sandbox.execute(&code, CodeLanguage::Bash).await
                });
                let _ = tx.send(result);
            });

            match rx.recv_timeout(std::time::Duration::from_secs(60)) {
                Ok(result) => Ok(Dynamic::from(result.output())),
                Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    "RUN BASH timed out".into(),
                    rhai::Position::NONE,
                ))),
            }
        })
    {
        log::error!("Failed to register RUN BASH syntax failed: {e}");
    }
}
pub fn run_file_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();

    if let Err(e) = engine
        .register_custom_syntax(
            ["RUN", "PYTHON", "WITH", "FILE", "$expr$"],
            false,
            move |context, inputs| {
                let file_path = context
                    .eval_expression_tree(&inputs[0])?
                    .to_string()
                    .trim_matches('"')
                    .to_string();

                trace!(
                    "RUN PYTHON WITH FILE {} for session: {}",
                    file_path,
                    user_clone.id
                );

                let state_for_task = Arc::clone(&state_clone);
                let session_id = user_clone.id;
                let bot_id = user_clone.bot_id;

                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let rt = match tokio::runtime::Runtime::new() {
                        Ok(rt) => rt,
                        Err(e) => {
                            let _ = tx.send(ExecutionResult::error(&format!("Failed to create runtime: {}", e)));
                            return;
                        }
                    };
                    let result = rt.block_on(async {
                        let config = SandboxConfig::from_bot_config(&*state_for_task, bot_id);
                        let sandbox = CodeSandbox::new(config, session_id);
                        sandbox.execute_file(&file_path, CodeLanguage::Python).await
                    });
                    let _ = tx.send(result);
                });

                match rx.recv_timeout(std::time::Duration::from_secs(60)) {
                    Ok(result) => Ok(Dynamic::from(result.output())),
                    Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "RUN PYTHON WITH FILE timed out".into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register RUN PYTHON WITH FILE syntax failed: {e}");
    }

    if let Err(e) = engine
        .register_custom_syntax(
            ["RUN", "JAVASCRIPT", "WITH", "FILE", "$expr$"],
            false,
            move |context, inputs| {
                let file_path = context
                    .eval_expression_tree(&inputs[0])?
                    .to_string()
                    .trim_matches('"')
                    .to_string();

                let state_for_task = Arc::clone(&state);
                let session_id = user.id;
                let bot_id = user.bot_id;

                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let rt = match tokio::runtime::Runtime::new() {
                        Ok(rt) => rt,
                        Err(e) => {
                            let _ = tx.send(ExecutionResult::error(&format!("Failed to create runtime: {}", e)));
                            return;
                        }
                    };
                    let result = rt.block_on(async {
                        let config = SandboxConfig::from_bot_config(&*state_for_task, bot_id);
                        let sandbox = CodeSandbox::new(config, session_id);
                        sandbox
                            .execute_file(&file_path, CodeLanguage::JavaScript)
                            .await
                    });
                    let _ = tx.send(result);
                });

                match rx.recv_timeout(std::time::Duration::from_secs(60)) {
                    Ok(result) => Ok(Dynamic::from(result.output())),
                    Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "RUN JAVASCRIPT WITH FILE timed out".into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register RUN JAVASCRIPT WITH FILE syntax failed: {e}");
    }
}
