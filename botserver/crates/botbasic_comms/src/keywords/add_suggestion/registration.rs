use super::*;

pub fn clear_suggestions_keyword(
    state: Arc<dyn BasicRuntime>,
    user_session: UserSession,
    engine: &mut Engine,
) {
    let cache = state.cache_client().clone();
    let cache2 = state.cache_client().clone();
    let user_session2 = user_session.clone();

    if let Err(e) = engine
        .register_custom_syntax(["CLEAR", "SUGGESTIONS"], true, move |_context, _inputs| {
            if let Some(cache_client) = &cache {
                // TODO(#477): Pass org when available (UserSession does not carry org yet)
                let redis_key = botlib::key_utils::build_key("", &["suggestions", &user_session.bot_id.to_string(), &user_session.id.to_string()]);
                let mut conn = match get_redis_connection(cache_client) {
                    Some(conn) => conn,
                    None => {
                        trace!("Cache not ready, skipping clear suggestions");
                        return Ok(Dynamic::UNIT);
                    }
                };

                let result: Result<i64, redis::RedisError> =
                    redis::cmd("DEL").arg(&redis_key).query(&mut conn);

                match result {
                    Ok(deleted) => {
                        trace!(
                            "Cleared {} suggestions from session {}",
                            deleted,
                            user_session.id
                        );
                    }
                    Err(e) => log::error!("Failed to clear suggestions from Redis: {}", e),
                }
            } else {
                trace!("No cache configured, suggestions not cleared");
            }

            Ok(Dynamic::UNIT)
        })
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    engine
        .register_fn("CLEAR_SUGGESTIONS", move || {
            if let Some(cache_client) = &cache2 {
                let redis_key = botlib::key_utils::build_key("", &["suggestions", &user_session2.bot_id.to_string(), &user_session2.id.to_string()]);
                if let Some(mut conn) = get_redis_connection(cache_client) {
                    let _: Result<i64, redis::RedisError> =
                        redis::cmd("DEL").arg(&redis_key).query(&mut conn);
                }
            }
        });
}

pub fn add_suggestion_keyword(
    state: Arc<dyn BasicRuntime>,
    user_session: UserSession,
    engine: &mut Engine,
) {
    // Each closure needs its own Arc<redis::Client> and UserSession clone
    let cache = state.cache_client().clone();
    let cache2 = state.cache_client().clone();
    let cache3 = state.cache_client().clone();
    let cache4 = state.cache_client().clone();
    let user_session = user_session.clone();
    let user_session2 = user_session.clone();
    let user_session3 = user_session.clone();
    let user_session4 = user_session.clone();

    // ADD_SUGGESTION_TOOL "tool_name" as "button text"
if let Err(e) =     engine
        .register_custom_syntax(
            ["ADD_SUGGESTION_TOOL", "$expr$", "as", "$expr$"],
            true,
            move |context, inputs| {
                let tool_name = context.eval_expression_tree(&inputs[0])?.to_string();
                let button_text = context.eval_expression_tree(&inputs[1])?.to_string();

                add_tool_suggestion(
                    cache.as_ref(),
                    &user_session,
                    &tool_name,
                    None,
                    &button_text,
                )?;

                Ok(Dynamic::UNIT)
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    // ADD_SUGGESTION_TEXT "text_value" as "button text"
if let Err(e) =     engine
        .register_custom_syntax(
            ["ADD_SUGGESTION_TEXT", "$expr$", "as", "$expr$"],
            true,
            move |context, inputs| {
                let text_value = context.eval_expression_tree(&inputs[0])?.to_string();
                let button_text = context.eval_expression_tree(&inputs[1])?.to_string();

                add_text_suggestion(cache2.as_ref(), &user_session2, &text_value, &button_text)?;

                Ok(Dynamic::UNIT)
            },
        )
        {
            log::error!("Failed to register the custom syntax: {e}");
        }

    // ADD_SUGGESTION "context_name" as "button text" (register BEFORE simple form so simple form has higher priority)
if let Err(e) =     engine
        .register_custom_syntax(
            ["ADD_SUGGESTION", "$expr$", "as", "$expr$"],
            true,
            move |context, inputs| {
                let context_name = context.eval_expression_tree(&inputs[0])?.to_string();
                let button_text = context.eval_expression_tree(&inputs[1])?.to_string();

                add_context_suggestion(
                    cache3.as_ref(),
                    &user_session3,
                    &context_name,
                    &button_text,
                )?;

                Ok(Dynamic::UNIT)
            },
        )
        {
            log::error!("Failed to register the custom syntax: {e}");
        }

    // ADD_SUGGESTION "button text" (simple form - sends message on click)
    // Registered LAST so it has HIGHEST priority — Rhai tries this first, falls back to 2-arg form
if let Err(e) =     engine
        .register_custom_syntax(
            ["ADD_SUGGESTION", "$expr$"],
            true,
            move |context, inputs| {
                let button_text = context.eval_expression_tree(&inputs[0])?.to_string();

                add_text_suggestion(
                    cache4.as_ref(),
                    &user_session4,
                    &button_text,
                    &button_text,
                )?;

                Ok(Dynamic::UNIT)
            },
        )
        {
            log::error!("Failed to register the custom syntax: {e}");
        }
}
