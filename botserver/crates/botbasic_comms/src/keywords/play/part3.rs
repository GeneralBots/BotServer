use super::*;

pub fn play_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    if let Err(e) = play_simple_keyword(Arc::clone(&state), user.clone(), engine) {
        log::error!("Failed to register PLAY keyword: {e}");
    }
    if let Err(e) = play_with_options_keyword(Arc::clone(&state), user.clone(), engine) {
        log::error!("Failed to register PLAY WITH OPTIONS keyword: {e}");
    }
    if let Err(e) = stop_keyword(Arc::clone(&state), user.clone(), engine) {
        log::error!("Failed to register STOP keyword: {e}");
    }
    if let Err(e) = pause_keyword(Arc::clone(&state), user.clone(), engine) {
        log::error!("Failed to register PAUSE keyword: {e}");
    }
    if let Err(e) = resume_keyword(Arc::clone(&state), user.clone(), engine) {
        log::error!("Failed to register RESUME keyword: {e}");
    }
    if let Err(e) = preview_keyword(state, user, engine) {
        log::error!("Failed to register PREVIEW keyword: {e}");
    }
}
pub(super) fn preview_keyword(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) -> Result<(), rhai::ParseError> {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    engine.register_custom_syntax(["PREVIEW", "$expr$"], false, move |context, inputs| {
        let content = context
            .eval_expression_tree(&inputs[0])?
            .to_string();

        let clean_content = if content.starts_with('"') && content.ends_with('"') {
            content[1..content.len()-1].to_string()
        } else {
            content
        };

        trace!("PREVIEW keyword for session: {}", user_clone.id);

        let state_for_task = Arc::clone(&state_clone);
        let session_id = user_clone.id;

        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let encoded = urlencoding::encode(&clean_content);
            let source_url = format!("data:text/html;charset=utf-8,{}", encoded);
            
            let response = PlayResponse {
                player_id: Uuid::new_v4(),
                content_type: ContentType::Html,
                component: ContentType::Html.player_component().to_string(),
                source_url,
                title: "Preview".to_string(),
                options: PlayOptions::default(),
                metadata: HashMap::new(),
            };

            let result = send_play_to_client(&state_for_task, session_id, &response);
            let _ = tx.send(result);
        });

        match rx.recv_timeout(std::time::Duration::from_secs(10)) {
            Ok(Ok(_)) => Ok(Dynamic::from("Preview displayed")),
            Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                e.into(),
                rhai::Position::NONE,
            ))),
            Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                "PREVIEW timed out".into(),
                rhai::Position::NONE,
            ))),
        }
    })?;
    Ok(())
}
pub(super) fn play_simple_keyword(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) -> Result<(), rhai::ParseError> {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    engine.register_custom_syntax(["PLAY", "$expr$"], false, move |context, inputs| {
        let source = context
            .eval_expression_tree(&inputs[0])?
            .to_string()
            .trim_matches('"')
            .to_string();

        trace!("PLAY '{source}' for session: {}", user_clone.id);

        let state_for_task = Arc::clone(&state_clone);
        let session_id = user_clone.id;

        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let result = execute_play(&state_for_task, session_id, &source, PlayOptions::default());
            let _ = tx.send(result);
        });

        match rx.recv_timeout(std::time::Duration::from_secs(30)) {
            Ok(Ok(response)) => {
                let json = serde_json::to_string(&response).unwrap_or_default();
                Ok(Dynamic::from(json))
            }
            Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                e.into(),
                rhai::Position::NONE,
            ))),
            Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                "PLAY timed out".into(),
                rhai::Position::NONE,
            ))),
        }
    })?;
    Ok(())
}
pub(super) fn play_with_options_keyword(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) -> Result<(), rhai::ParseError> {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    engine.register_custom_syntax(
        ["PLAY", "$expr$", "WITH", "OPTIONS", "$expr$"],
        false,
        move |context, inputs| {
            let source = context
                .eval_expression_tree(&inputs[0])?
                .to_string()
                .trim_matches('"')
                .to_string();
            let options_str = context
                .eval_expression_tree(&inputs[1])?
                .to_string()
                .trim_matches('"')
                .to_string();

            let options = PlayOptions::from_string(&options_str);

            trace!(
                "PLAY '{source}' WITH OPTIONS '{options_str}' for session: {}",
                user_clone.id
            );

            let state_for_task = Arc::clone(&state_clone);
            let session_id = user_clone.id;

            let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let result = execute_play(&state_for_task, session_id, &source, options);
            let _ = tx.send(result);
        });

            match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                Ok(Ok(response)) => {
                    let json = serde_json::to_string(&response).unwrap_or_default();
                    Ok(Dynamic::from(json))
                }
                Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    e.into(),
                    rhai::Position::NONE,
                ))),
                Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    "PLAY timed out".into(),
                    rhai::Position::NONE,
                ))),
            }
        },
    )?;
    Ok(())
}
pub(super) fn stop_keyword(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) -> Result<(), rhai::ParseError> {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    engine.register_custom_syntax(["STOP"], false, move |_context, _inputs| {
        trace!("STOP playback for session: {}", user_clone.id);

        let state_for_task = Arc::clone(&state_clone);
        let session_id = user_clone.id;

        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let result = send_player_command(&state_for_task, session_id, "stop");
            let _ = tx.send(result);
        });

        match rx.recv_timeout(std::time::Duration::from_secs(10)) {
            Ok(Ok(_)) => Ok(Dynamic::from("Playback stopped")),
            Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                e.into(),
                rhai::Position::NONE,
            ))),
            Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                "STOP timed out".into(),
                rhai::Position::NONE,
            ))),
        }
    })?;
    Ok(())
}
pub(super) fn pause_keyword(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) -> Result<(), rhai::ParseError> {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    engine.register_custom_syntax(["PAUSE"], false, move |_context, _inputs| {
        trace!("PAUSE playback for session: {}", user_clone.id);

        let state_for_task = Arc::clone(&state_clone);
        let session_id = user_clone.id;

        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let result = send_player_command(&state_for_task, session_id, "pause");
            let _ = tx.send(result);
        });

        match rx.recv_timeout(std::time::Duration::from_secs(10)) {
            Ok(Ok(_)) => Ok(Dynamic::from("Playback paused")),
            Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                e.into(),
                rhai::Position::NONE,
            ))),
            Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                "PAUSE timed out".into(),
                rhai::Position::NONE,
            ))),
        }
    })?;
    Ok(())
}
pub(super) fn resume_keyword(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) -> Result<(), rhai::ParseError> {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    engine.register_custom_syntax(["RESUME"], false, move |_context, _inputs| {
        trace!("RESUME playback for session: {}", user_clone.id);

        let state_for_task = Arc::clone(&state_clone);
        let session_id = user_clone.id;

        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let result = send_player_command(&state_for_task, session_id, "resume");
            let _ = tx.send(result);
        });

        match rx.recv_timeout(std::time::Duration::from_secs(10)) {
            Ok(Ok(_)) => Ok(Dynamic::from("Playback resumed")),
            Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                e.into(),
                rhai::Position::NONE,
            ))),
            Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                "RESUME timed out".into(),
                rhai::Position::NONE,
            ))),
        }
    })?;
    Ok(())
}
