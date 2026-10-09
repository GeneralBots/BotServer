use super::*;

pub fn get_instagram_metrics_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine
        .register_custom_syntax(
            ["GET", "INSTAGRAM", "METRICS", "$expr$"],
            false,
            move |context, inputs| {
                let post_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let post_id = post_id.trim_matches('"');

                trace!("GET INSTAGRAM METRICS: {}", post_id);

                let state_for_task = Arc::clone(&state_clone);
                let user_for_task = user_clone.clone();
                let post_id_owned = post_id.to_string();

                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    if let Ok(rt) = rt {
                        let result = rt.block_on(async move {
                            fetch_instagram_metrics(&*state_for_task, &user_for_task, &post_id_owned)
                                .await
                        });
                        let _ = tx.send(result);
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                    Ok(Ok(metrics)) => Ok(metrics.to_dynamic()),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("GET INSTAGRAM METRICS failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                    Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "GET INSTAGRAM METRICS timed out".into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    debug!("Registered GET INSTAGRAM METRICS keyword");
}

pub fn get_facebook_metrics_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine
        .register_custom_syntax(
            ["GET", "FACEBOOK", "METRICS", "$expr$"],
            false,
            move |context, inputs| {
                let post_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let post_id = post_id.trim_matches('"');

                trace!("GET FACEBOOK METRICS: {}", post_id);

                let state_for_task = Arc::clone(&state_clone);
                let user_for_task = user_clone.clone();
                let post_id_owned = post_id.to_string();

                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    if let Ok(rt) = rt {
                        let result = rt.block_on(async move {
                            fetch_facebook_metrics(&*state_for_task, &user_for_task, &post_id_owned)
                                .await
                        });
                        let _ = tx.send(result);
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                    Ok(Ok(metrics)) => Ok(metrics.to_dynamic()),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("GET FACEBOOK METRICS failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                    Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "GET FACEBOOK METRICS timed out".into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    debug!("Registered GET FACEBOOK METRICS keyword");
}

pub fn get_linkedin_metrics_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine
        .register_custom_syntax(
            ["GET", "LINKEDIN", "METRICS", "$expr$"],
            false,
            move |context, inputs| {
                let post_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let post_id = post_id.trim_matches('"');

                trace!("GET LINKEDIN METRICS: {}", post_id);

                let state_for_task = Arc::clone(&state_clone);
                let user_for_task = user_clone.clone();
                let post_id_owned = post_id.to_string();

                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    if let Ok(rt) = rt {
                        let result = rt.block_on(async move {
                            fetch_linkedin_metrics(&*state_for_task, &user_for_task, &post_id_owned)
                                .await
                        });
                        let _ = tx.send(result);
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                    Ok(Ok(metrics)) => Ok(metrics.to_dynamic()),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("GET LINKEDIN METRICS failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                    Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "GET LINKEDIN METRICS timed out".into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    debug!("Registered GET LINKEDIN METRICS keyword");
}

pub fn get_twitter_metrics_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine
        .register_custom_syntax(
            ["GET", "TWITTER", "METRICS", "$expr$"],
            false,
            move |context, inputs| {
                let post_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let post_id = post_id.trim_matches('"');

                trace!("GET TWITTER METRICS: {}", post_id);

                let state_for_task = Arc::clone(&state_clone);
                let user_for_task = user_clone.clone();
                let post_id_owned = post_id.to_string();

                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    if let Ok(rt) = rt {
                        let result = rt.block_on(async move {
                            fetch_twitter_metrics(&*state_for_task, &user_for_task, &post_id_owned)
                                .await
                        });
                        let _ = tx.send(result);
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                    Ok(Ok(metrics)) => Ok(metrics.to_dynamic()),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("GET TWITTER METRICS failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                    Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "GET TWITTER METRICS timed out".into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    debug!("Registered GET TWITTER METRICS keyword");
}
