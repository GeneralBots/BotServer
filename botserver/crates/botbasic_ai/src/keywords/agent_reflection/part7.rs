use super::*;

pub fn register_reflection_keywords(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    set_bot_reflection_keyword(state.clone(), user.clone(), engine);
    reflect_on_keyword(state.clone(), user.clone(), engine);
    get_reflection_insights_keyword(state, user, engine);
}
pub fn set_bot_reflection_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let _ = (&state, &user); // Mark as intentionally unused in registration
    let user_clone = user;

    if let Err(e) = engine
        .register_custom_syntax(
            ["SET", "BOT", "REFLECTION", "$expr$"],
            false,
            move |context, inputs| {
                let value = context
                    .eval_expression_tree(&inputs[0])?
                    .to_string()
                    .to_lowercase();

                let enabled = value == "true" || value == "1" || value == "on";

                trace!(
                    "SET BOT REFLECTION {} for bot: {}",
                    enabled,
                    user_clone.bot_id
                );

                let bot_id = user_clone.bot_id;

                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let _rt = match tokio::runtime::Runtime::new() {
                        Ok(rt) => rt,
                        Err(e) => {
                            let _ = tx.send(Err(format!("Failed to create runtime: {}", e)));
                            return;
                        }
                    };
                    let result = set_reflection_enabled(bot_id, enabled);
                    let _ = tx.send(result);
                });

                match rx.recv_timeout(std::time::Duration::from_secs(10)) {
                    Ok(Ok(msg)) => Ok(Dynamic::from(msg)),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        e.into(),
                        rhai::Position::NONE,
                    ))),
                    Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "SET BOT REFLECTION timed out".into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register SET BOT REFLECTION syntax failed: {e}");
    }
}
pub fn reflect_on_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine
        .register_custom_syntax(
            ["REFLECT", "ON", "$expr$"],
            false,
            move |context, inputs| {
                let reflection_type_str = context
                    .eval_expression_tree(&inputs[0])?
                    .to_string()
                    .trim_matches('"')
                    .to_string();

                let reflection_type = ReflectionType::from(reflection_type_str.as_str());

                trace!(
                    "REFLECT ON {:?} for session: {}",
                    reflection_type,
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
                            let _ = tx.send(Err(format!("Failed to create runtime: {}", e)));
                            return;
                        }
                    };
                    let result = rt.block_on(async {
                        let engine = ReflectionEngine::new(state_for_task, bot_id);
                        engine.reflect(session_id, reflection_type).await
                    });
                    let _ = tx.send(result);
                });

                match rx.recv_timeout(std::time::Duration::from_secs(60)) {
                    Ok(Ok(result)) => Ok(Dynamic::from(result.summary())),
                    Ok(Err(e)) => Ok(Dynamic::from(format!("Reflection failed: {}", e))),
                    Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "REFLECT ON timed out".into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register REFLECT ON syntax failed: {e}");
    }
}
pub fn get_reflection_insights_keyword(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    engine.register_fn("GET REFLECTION INSIGHTS", move || -> rhai::Array {
        let state = Arc::clone(&state_clone);
        let bot_id = user_clone.bot_id;

        let _rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                log::error!("Failed to create runtime: {}", e);
                return rhai::Array::new();
            }
        };
        let result = {
            let engine = ReflectionEngine::new(state, bot_id);
            engine.get_insights(10)
        };

        match result {
            Ok(insights) => insights
                .into_iter()
                .map(|i| Dynamic::from(i.summary()))
                .collect(),
            Err(_) => rhai::Array::new(),
        }
    });
}
pub(super) fn set_reflection_enabled(bot_id: Uuid, enabled: bool) -> Result<String, String> {
    let value = if enabled { "true" } else { "false" };
    write_bot_config_to_vault(&bot_id, "bot-reflection-enabled", value)
        .map_err(|e| format!("Failed to set reflection enabled: {}", e))?;

    Ok(format!(
        "Bot reflection {}",
        if enabled { "enabled" } else { "disabled" }
    ))
}
