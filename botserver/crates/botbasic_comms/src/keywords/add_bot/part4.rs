use super::*;

pub(super) fn list_bots_keyword(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) -> Result<(), rhai::ParseError> {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    engine.register_custom_syntax(["LIST", "BOTS"], false, move |_context, _inputs| {
        trace!("LIST BOTS for session: {}", user_clone.id);

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
            let result = rt.block_on(async { get_session_bots(&*state_for_task, session_id) });
            let _ = tx.send(result);
        });

        match rx.recv_timeout(std::time::Duration::from_secs(30)) {
            Ok(Ok(bots)) => {
                let bot_list: Vec<Dynamic> = bots
                    .into_iter()
                    .map(|b| {
                        let mut map = rhai::Map::new();
                        map.insert("name".into(), Dynamic::from(b.bot_name));
                        map.insert("priority".into(), Dynamic::from(b.priority));
                        map.insert(
                            "trigger_type".into(),
                            Dynamic::from(b.trigger.trigger_type.to_string()),
                        );
                        map.insert("is_active".into(), Dynamic::from(b.is_active));
                        Dynamic::from(map)
                    })
                    .collect();
                Ok(Dynamic::from(bot_list))
            }
            Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                e.into(),
                rhai::Position::NONE,
            ))),
            Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                "LIST BOTS timed out".into(),
                rhai::Position::NONE,
            ))),
        }
    })?;
    Ok(())
}
pub(super) fn set_bot_priority_keyword(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) -> Result<(), rhai::ParseError> {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    engine.register_custom_syntax(
        ["SET", "BOT", "PRIORITY", "$expr$", ",", "$expr$"],
        false,
        move |context, inputs| {
            let bot_name = context
                .eval_expression_tree(&inputs[0])?
                .to_string()
                .trim_matches('"')
                .to_string();
            let priority = context
                .eval_expression_tree(&inputs[1])?
                .as_int()
                .unwrap_or(0) as i32;

            trace!(
                "SET BOT PRIORITY '{bot_name}' to {priority} for session: {}",
                user_clone.id
            );

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
                    set_bot_priority(&*state_for_task, session_id, &bot_name, priority)
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
                    "SET BOT PRIORITY timed out".into(),
                    rhai::Position::NONE,
                ))),
            }
        },
    )?;
    Ok(())
}
pub(super) fn delegate_to_keyword(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) -> Result<(), rhai::ParseError> {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    engine.register_custom_syntax(
        ["DELEGATE", "TO", "$expr$"],
        false,
        move |context, inputs| {
            let bot_name = context
                .eval_expression_tree(&inputs[0])?
                .to_string()
                .trim_matches('"')
                .to_string();

            trace!("DELEGATE TO '{bot_name}' for session: {}", user_clone.id);

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
                let result =
                    rt.block_on(async { delegate_to_bot(&*state_for_task, session_id, &bot_name) });
                let _ = tx.send(result);
            });

            match rx.recv_timeout(std::time::Duration::from_secs(60)) {
                Ok(Ok(response)) => Ok(Dynamic::from(response)),
                Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    e.into(),
                    rhai::Position::NONE,
                ))),
                Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    "DELEGATE TO timed out".into(),
                    rhai::Position::NONE,
                ))),
            }
        },
    )?;
    Ok(())
}
pub(super) fn add_bot_to_session(
    state: &dyn BasicRuntime,
    session_id: Uuid,
    _parent_bot_id: Uuid,
    bot_name: &str,
    trigger: BotTrigger,
) -> Result<String, String> {
    let mut conn = state.db_pool().get().map_err(|e| format!("DB error: {e}"))?;

    let bot_exists: bool = diesel::sql_query(
        "SELECT EXISTS(SELECT 1 FROM bots WHERE name = $1 AND is_active = true) as exists",
    )
    .bind::<diesel::sql_types::Text, _>(bot_name)
    .get_result::<BoolResult>(&mut *conn)
    .map(|r| r.exists)
    .unwrap_or(false);

    let bot_id: String = if bot_exists {
        diesel::sql_query("SELECT id FROM bots WHERE name = $1 AND is_active = true")
            .bind::<diesel::sql_types::Text, _>(bot_name)
            .get_result::<UuidResult>(&mut *conn)
            .map(|r| r.id)
            .map_err(|e| format!("Failed to get bot ID: {e}"))?
    } else {
        return Err(format!(
            "Bot '{}' does not exist in database. Please create it first using the import process.",
            bot_name
        ));
    };

    let trigger_json =
        serde_json::to_string(&trigger).map_err(|e| format!("Failed to serialize trigger: {e}"))?;

    let association_id = Uuid::new_v4();
    diesel::sql_query(
        "INSERT INTO session_bots (id, session_id, bot_id, bot_name, trigger_config, priority, is_active, joined_at)
         VALUES ($1, $2, $3, $4, $5, 0, true, NOW())
         ON CONFLICT (session_id, bot_name)
         DO UPDATE SET trigger_config = $5, is_active = true, joined_at = NOW()",
    )
    .bind::<diesel::sql_types::Text, _>(association_id.to_string())
    .bind::<diesel::sql_types::Text, _>(session_id.to_string())
    .bind::<diesel::sql_types::Text, _>(bot_id)
    .bind::<diesel::sql_types::Text, _>(bot_name)
    .bind::<diesel::sql_types::Text, _>(&trigger_json)
    .execute(&mut *conn)
    .map_err(|e| format!("Failed to add bot to session: {e}"))?;

    info!(
        "Bot '{bot_name}' added to session {session_id} with trigger type: {:?}",
        trigger.trigger_type
    );

    Ok(format!("Bot '{bot_name}' added to conversation"))
}
pub(super) fn remove_bot_from_session(
    state: &dyn BasicRuntime,
    session_id: Uuid,
    bot_name: &str,
) -> Result<String, String> {
    let mut conn = state.db_pool().get().map_err(|e| format!("DB error: {e}"))?;

    let affected = diesel::sql_query(
        "UPDATE session_bots SET is_active = false WHERE session_id = $1 AND bot_name = $2",
    )
    .bind::<diesel::sql_types::Text, _>(session_id.to_string())
    .bind::<diesel::sql_types::Text, _>(bot_name)
    .execute(&mut conn)
    .map_err(|e| format!("Failed to remove bot: {e}"))?;

    if affected > 0 {
        info!("Bot '{bot_name}' removed from session {session_id}");
        Ok(format!("Bot '{bot_name}' removed from conversation"))
    } else {
        Ok(format!("Bot '{bot_name}' was not in the conversation"))
    }
}
pub(super) fn get_session_bots(state: &dyn BasicRuntime, session_id: Uuid) -> Result<Vec<SessionBot>, String> {
    let mut conn = state.db_pool().get().map_err(|e| format!("DB error: {e}"))?;

    let results: Vec<SessionBotRow> = diesel::sql_query(
        "SELECT id, session_id, bot_id, bot_name, trigger_config, priority, is_active
         FROM session_bots
         WHERE session_id = $1 AND is_active = true
         ORDER BY priority DESC, joined_at ASC",
    )
    .bind::<diesel::sql_types::Text, _>(session_id.to_string())
    .load(&mut *conn)
    .map_err(|e| format!("Failed to get session bots: {e}"))?;

    let bots = results
        .into_iter()
        .filter_map(|row| {
            let trigger: BotTrigger =
                serde_json::from_str(&row.trigger_config).unwrap_or_else(|_| BotTrigger::always());
            Some(SessionBot {
                id: Uuid::parse_str(&row.id).ok()?,
                session_id: Uuid::parse_str(&row.session_id).ok()?,
                bot_id: Uuid::parse_str(&row.bot_id).ok()?,
                bot_name: row.bot_name,
                trigger,
                priority: row.priority,
                is_active: row.is_active,
            })
        })
        .collect();

    Ok(bots)
}
pub(super) fn set_bot_priority(
    state: &dyn BasicRuntime,
    session_id: Uuid,
    bot_name: &str,
    priority: i32,
) -> Result<String, String> {
    let mut conn = state.db_pool().get().map_err(|e| format!("DB error: {e}"))?;

    diesel::sql_query(
        "UPDATE session_bots SET priority = $1 WHERE session_id = $2 AND bot_name = $3",
    )
    .bind::<diesel::sql_types::Integer, _>(priority)
    .bind::<diesel::sql_types::Text, _>(session_id.to_string())
    .bind::<diesel::sql_types::Text, _>(bot_name)
    .execute(&mut *conn)
    .map_err(|e| format!("Failed to set priority: {e}"))?;

    Ok(format!("Bot '{bot_name}' priority set to {priority}"))
}
