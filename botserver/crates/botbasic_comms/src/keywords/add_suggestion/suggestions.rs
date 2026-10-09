use super::*;

pub(super) fn add_context_suggestion(
    cache: Option<&Arc<redis::Client>>,
    user_session: &UserSession,
    context_name: &str,
    button_text: &str,
) -> Result<(), Box<rhai::EvalAltResult>> {
    if let Some(cache_client) = cache {
        // TODO(#477): Pass org when available (UserSession does not carry org yet)
        let redis_key = botlib::key_utils::build_key("", &["suggestions", &user_session.bot_id.to_string(), &user_session.id.to_string()]);

        let suggestion = json!({
            "type": "context",
            "context": context_name,
            "text": button_text,
            "action": {
                "type": "select_context",
                "context": context_name
            }
        });

        let mut conn = match get_redis_connection(cache_client) {
            Some(conn) => conn,
            None => {
                trace!("Cache not ready, skipping add context suggestion");
                return Ok(());
            }
        };

        let _: Result<i64, redis::RedisError> = redis::cmd("SADD")
            .arg(&redis_key)
            .arg(suggestion.to_string())
            .query(&mut conn);

        trace!(
            "Added context suggestion '{}' to session {}",
            context_name,
            user_session.id
        );

        // TODO(#477): Pass org when available (UserSession does not carry org yet)
        let active_key = botlib::key_utils::build_key("", &["active_context", &user_session.bot_id.to_string(), &user_session.id.to_string()]);

        let _: Result<i64, redis::RedisError> = redis::cmd("HSET")
            .arg(&active_key)
            .arg(context_name)
            .arg("inactive")
            .query(&mut conn);
    } else {
        trace!("No cache configured, suggestion not added");
    }

    Ok(())
}

pub(super) fn add_text_suggestion(
    cache: Option<&Arc<redis::Client>>,
    user_session: &UserSession,
    text_value: &str,
    button_text: &str,
) -> Result<(), Box<rhai::EvalAltResult>> {
    if let Some(cache_client) = cache {
        // TODO(#477): Pass org when available (UserSession does not carry org yet)
        let redis_key = botlib::key_utils::build_key("", &["suggestions", &user_session.bot_id.to_string(), &user_session.id.to_string()]);

        let suggestion = json!({
            "type": "text_value",
            "text": button_text,
            "value": text_value,
            "action": {
                "type": "send_message",
                "message": text_value
            }
        });

        let mut conn = match get_redis_connection(cache_client) {
            Some(conn) => conn,
            None => {
                trace!("Cache not ready, skipping add text suggestion");
                return Ok(());
            }
        };

        let _: Result<i64, redis::RedisError> = redis::cmd("SADD")
            .arg(&redis_key)
            .arg(suggestion.to_string())
            .query(&mut conn);

        trace!(
            "Added text suggestion '{}' to session {}",
            text_value,
            user_session.id
        );
    } else {
        trace!("No cache configured, text suggestion not added");
    }

    Ok(())
}

pub(super) fn add_tool_suggestion(
    cache: Option<&Arc<redis::Client>>,
    user_session: &UserSession,
    tool_name: &str,
    params: Option<Vec<String>>,
    button_text: &str,
) -> Result<(), Box<rhai::EvalAltResult>> {
    info!(
        "ADD_SUGGESTION_TOOL called: tool={}, button={}",
        tool_name, button_text
    );
    if let Some(cache_client) = cache {
        // TODO(#477): Pass org when available (UserSession does not carry org yet)
        let redis_key = botlib::key_utils::build_key("", &["suggestions", &user_session.bot_id.to_string(), &user_session.id.to_string()]);
        info!("Adding suggestion to Redis key: {}", redis_key);

        let prompt_for_params = params.as_ref().is_some_and(|p| !p.is_empty());
        let action_obj = json!({
            "type": "invoke_tool",
            "tool": tool_name,
            "params": params,
            "prompt_for_params": prompt_for_params
        });

        let suggestion = json!({
            "type": "invoke_tool",
            "text": button_text,
            "tool": tool_name,
            "action": action_obj
        });

        let mut conn = match get_redis_connection(cache_client) {
            Some(conn) => conn,
            None => {
                trace!("Cache not ready, skipping add tool suggestion");
                return Ok(());
            }
        };

        let _: Result<i64, redis::RedisError> = redis::cmd("SADD")
            .arg(&redis_key)
            .arg(suggestion.to_string())
            .query(&mut conn);

        info!(
            "Added tool suggestion '{}' to session {}",
            tool_name, user_session.id
        );
    } else {
        trace!("No cache configured, tool suggestion not added");
    }

    Ok(())
}
