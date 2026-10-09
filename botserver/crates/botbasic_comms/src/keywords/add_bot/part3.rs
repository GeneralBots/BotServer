use super::*;

pub fn register_bot_keywords(state: &Arc<dyn BasicRuntime>, user: &UserSession, engine: &mut Engine) {
    if let Err(e) = add_bot_with_trigger_keyword(Arc::clone(state), user.clone(), engine) {
        log::error!("Failed to register ADD BOT WITH TRIGGER keyword: {e}");
    }
    if let Err(e) = add_bot_with_tools_keyword(Arc::clone(state), user.clone(), engine) {
        log::error!("Failed to register ADD BOT WITH TOOLS keyword: {e}");
    }
    if let Err(e) = add_bot_with_schedule_keyword(Arc::clone(state), user.clone(), engine) {
        log::error!("Failed to register ADD BOT WITH SCHEDULE keyword: {e}");
    }
    if let Err(e) = remove_bot_keyword(Arc::clone(state), user.clone(), engine) {
        log::error!("Failed to register REMOVE BOT keyword: {e}");
    }
    if let Err(e) = list_bots_keyword(Arc::clone(state), user.clone(), engine) {
        log::error!("Failed to register LIST BOTS keyword: {e}");
    }
    if let Err(e) = set_bot_priority_keyword(Arc::clone(state), user.clone(), engine) {
        log::error!("Failed to register SET BOT PRIORITY keyword: {e}");
    }
    if let Err(e) = delegate_to_keyword(Arc::clone(state), user.clone(), engine) {
        log::error!("Failed to register DELEGATE TO keyword: {e}");
    }
}
pub(super) fn add_bot_with_trigger_keyword(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) -> Result<(), rhai::ParseError> {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    engine.register_custom_syntax(
        ["ADD", "BOT", "$expr$", "WITH", "TRIGGER", "$expr$"],
        false,
        move |context, inputs| {
            let bot_name = context
                .eval_expression_tree(&inputs[0])?
                .to_string()
                .trim_matches('"')
                .to_string();
            let trigger_str = context
                .eval_expression_tree(&inputs[1])?
                .to_string()
                .trim_matches('"')
                .to_string();

            trace!(
                "ADD BOT '{bot_name}' WITH TRIGGER '{trigger_str}' for session: {}",
                user_clone.id
            );

            let keywords: Vec<String> = trigger_str
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect();

            let trigger = BotTrigger::from_keywords(keywords);
            let state_for_task = Arc::clone(&state_clone);
            let session_id = user_clone.id;
            let bot_id = user_clone.bot_id;

            let (tx, rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        let _ = tx.send(Err(format!("Failed to create runtime: {e}")));
                        return;
                    }
                };
                let result = rt.block_on(async {
                    add_bot_to_session(&*state_for_task, session_id, bot_id, &bot_name, trigger)
                });
                let _ = tx.send(result);
            });

            match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                Ok(Ok(msg)) => Ok(Dynamic::from(msg)),
                Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    e.into(),
                    rhai::Position::NONE,
                ))),
                Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    "ADD BOT timed out".into(),
                    rhai::Position::NONE,
                ))),
            }
        },
    )?;
    Ok(())
}
pub(super) fn add_bot_with_tools_keyword(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) -> Result<(), rhai::ParseError> {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    engine.register_custom_syntax(
        ["ADD", "BOT", "$expr$", "WITH", "TOOLS", "$expr$"],
        false,
        move |context, inputs| {
            let bot_name = context
                .eval_expression_tree(&inputs[0])?
                .to_string()
                .trim_matches('"')
                .to_string();
            let tools_str = context
                .eval_expression_tree(&inputs[1])?
                .to_string()
                .trim_matches('"')
                .to_string();

            trace!(
                "ADD BOT '{bot_name}' WITH TOOLS '{tools_str}' for session: {}",
                user_clone.id
            );

            let tools: Vec<String> = tools_str
                .split(',')
                .map(|s| s.trim().to_uppercase())
                .filter(|s| !s.is_empty())
                .collect();

            let trigger = BotTrigger::from_tools(tools);
            let state_for_task = Arc::clone(&state_clone);
            let session_id = user_clone.id;
            let bot_id = user_clone.bot_id;

            let (tx, rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        let _ = tx.send(Err(format!("Failed to create runtime: {e}")));
                        return;
                    }
                };
                let result = rt.block_on(async {
                    add_bot_to_session(&*state_for_task, session_id, bot_id, &bot_name, trigger)
                });
                let _ = tx.send(result);
            });

            match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                Ok(Ok(msg)) => Ok(Dynamic::from(msg)),
                Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    e.into(),
                    rhai::Position::NONE,
                ))),
                Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    "ADD BOT timed out".into(),
                    rhai::Position::NONE,
                ))),
            }
        },
    )?;
    Ok(())
}
pub(super) fn add_bot_with_schedule_keyword(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) -> Result<(), rhai::ParseError> {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    engine.register_custom_syntax(
        ["ADD", "BOT", "$expr$", "WITH", "SCHEDULE", "$expr$"],
        false,
        move |context, inputs| {
            let bot_name = context
                .eval_expression_tree(&inputs[0])?
                .to_string()
                .trim_matches('"')
                .to_string();
            let schedule = context
                .eval_expression_tree(&inputs[1])?
                .to_string()
                .trim_matches('"')
                .to_string();

            trace!(
                "ADD BOT '{bot_name}' WITH SCHEDULE '{schedule}' for session: {}",
                user_clone.id
            );

            let trigger = BotTrigger::from_schedule(schedule);
            let state_for_task = Arc::clone(&state_clone);
            let session_id = user_clone.id;
            let bot_id = user_clone.bot_id;

            let (tx, rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        let _ = tx.send(Err(format!("Failed to create runtime: {e}")));
                        return;
                    }
                };
                let result = rt.block_on(async {
                    add_bot_to_session(&*state_for_task, session_id, bot_id, &bot_name, trigger)
                });
                let _ = tx.send(result);
            });

            match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                Ok(Ok(msg)) => Ok(Dynamic::from(msg)),
                Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    e.into(),
                    rhai::Position::NONE,
                ))),
                Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    "ADD BOT timed out".into(),
                    rhai::Position::NONE,
                ))),
            }
        },
    )?;
    Ok(())
}
pub(super) fn remove_bot_keyword(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) -> Result<(), rhai::ParseError> {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    engine.register_custom_syntax(
        ["REMOVE", "BOT", "$expr$"],
        false,
        move |context, inputs| {
            let bot_name = context
                .eval_expression_tree(&inputs[0])?
                .to_string()
                .trim_matches('"')
                .to_string();

            trace!("REMOVE BOT '{bot_name}' from session: {}", user_clone.id);

            let state_for_task = Arc::clone(&state_clone);
            let session_id = user_clone.id;

            let (tx, rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        let _ = tx.send(Err(format!("Failed to create runtime: {e}")));
                        return;
                    }
                };
                let result = rt.block_on(async {
                    remove_bot_from_session(&*state_for_task, session_id, &bot_name)
                });
                let _ = tx.send(result);
            });

            match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                Ok(Ok(msg)) => Ok(Dynamic::from(msg)),
                Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    e.into(),
                    rhai::Position::NONE,
                ))),
                Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    "REMOVE BOT timed out".into(),
                    rhai::Position::NONE,
                ))),
            }
        },
    )?;
    Ok(())
}
