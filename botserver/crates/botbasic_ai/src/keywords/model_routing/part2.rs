use super::*;

pub(super) fn set_session_model(
    state: &Arc<dyn BasicRuntime>,
    session_id: Uuid,
    model_name: &str,
) -> Result<String, String> {
    let mut conn = state
        .db_pool()
        .get()
        .map_err(|e| format!("Failed to acquire database connection: {}", e))?;

    let now = chrono::Utc::now();

    diesel::sql_query(
        "INSERT INTO session_preferences (session_id, preference_key, preference_value, updated_at) \
         VALUES ($1, 'current_model', $2, $3) \
         ON CONFLICT (session_id, preference_key) DO UPDATE SET \
         preference_value = EXCLUDED.preference_value, \
         updated_at = EXCLUDED.updated_at",
    )
    .bind::<diesel::sql_types::Uuid, _>(session_id)
    .bind::<diesel::sql_types::Text, _>(model_name)
    .bind::<diesel::sql_types::Timestamptz, _>(now)
    .execute(&mut conn)
    .map_err(|e| format!("Failed to set session model: {}", e))?;

    info!("Session {} now using model: {}", session_id, model_name);

    Ok(format!("Now using model: {}", model_name))
}
pub(super) fn set_session_routing_strategy(
    state: &Arc<dyn BasicRuntime>,
    session_id: Uuid,
    strategy: RoutingStrategy,
) -> Result<String, String> {
    let mut conn = state
        .db_pool()
        .get()
        .map_err(|e| format!("Failed to acquire database connection: {}", e))?;

    let now = chrono::Utc::now();
    let strategy_str = match strategy {
        RoutingStrategy::Manual => "manual",
        RoutingStrategy::Auto => "auto",
        RoutingStrategy::LoadBalanced => "load-balanced",
        RoutingStrategy::Fallback => "fallback",
    };

    diesel::sql_query(
        "INSERT INTO session_preferences (session_id, preference_key, preference_value, updated_at) \
         VALUES ($1, 'model_routing', $2, $3) \
         ON CONFLICT (session_id, preference_key) DO UPDATE SET \
         preference_value = EXCLUDED.preference_value, \
         updated_at = EXCLUDED.updated_at",
    )
    .bind::<diesel::sql_types::Uuid, _>(session_id)
    .bind::<diesel::sql_types::Text, _>(strategy_str)
    .bind::<diesel::sql_types::Timestamptz, _>(now)
    .execute(&mut conn)
    .map_err(|e| format!("Failed to set routing strategy: {}", e))?;

    info!(
        "Session {} routing strategy set to: {}",
        session_id, strategy_str
    );

    Ok(format!("Model routing set to: {}", strategy_str))
}
pub(super) fn get_session_model_sync(
    conn: &mut diesel::PgConnection,
    session_id: Uuid,
) -> Result<String, String> {
    #[derive(QueryableByName)]
    struct PrefValue {
        #[diesel(sql_type = diesel::sql_types::Text)]
        preference_value: String,
    }

    // 1. Check session preference first (set by USE MODEL)
    let result: Option<PrefValue> = diesel::sql_query(
        "SELECT preference_value FROM session_preferences \
         WHERE session_id = $1 AND preference_key = 'current_model' LIMIT 1",
    )
    .bind::<diesel::sql_types::Uuid, _>(session_id)
    .get_result(conn)
    .optional()
    .map_err(|e| format!("Failed to get session model: {}", e))?;

    if let Some(pref) = result {
        return Ok(pref.preference_value);
    }

    // 2. No session preference - get bot's configured model from Vault
    // Need to get bot_id from session first
    #[derive(QueryableByName)]
    struct SessionBot {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        bot_id: Uuid,
    }

    let bot_result: Option<SessionBot> = diesel::sql_query(
        "SELECT bot_id FROM sessions WHERE id = $1 LIMIT 1",
    )
    .bind::<diesel::sql_types::Uuid, _>(session_id)
    .get_result(conn)
    .optional()
    .map_err(|e| format!("Failed to get session bot: {}", e))?;

    if let Some(session_bot) = bot_result {
        let bot_id = session_bot.bot_id;

        // Get bot's llm-model config from Vault
        if let Some(vault_configs) = read_bot_config_from_vault(&bot_id) {
            if let Some(model) = vault_configs.get("llm-model") {
                if !model.is_empty() && model != "true" {
                    return Ok(model.clone());
                }
            }
        }

    // 3. Bot has no model configured - fall back to default bot's model from Vault
    let default_bot_uuid: uuid::Uuid = uuid::Uuid::nil();

        if let Some(vault_configs) = read_bot_config_from_vault(&default_bot_uuid) {
            if let Some(model) = vault_configs.get("llm-model") {
                if !model.is_empty() && model != "true" {
                    return Ok(model.clone());
                }
            }
        }
    }

    // 4. Ultimate fallback
    Ok("llama-3.3:8b".to_string())
}
pub(super) fn list_available_models_sync(
    _conn: &mut diesel::PgConnection,
    bot_id: Uuid,
) -> Result<Vec<String>, String> {
    if let Some(vault_configs) = read_bot_config_from_vault(&bot_id) {
        if let Some(config_value) = vault_configs.get("llm-models") {
            return Ok(config_value
                .split(';')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect());
        }
    }

    Ok(vec!["default".to_string()])
}
pub fn get_session_model(state: &Arc<dyn BasicRuntime>, session_id: Uuid) -> String {
    if let Ok(mut conn) = state.db_pool().get() {
        get_session_model_sync(&mut conn, session_id).unwrap_or_else(|_| "default".to_string())
    } else {
        "default".to_string()
    }
}
pub fn get_session_routing_strategy(state: &Arc<dyn BasicRuntime>, session_id: Uuid) -> RoutingStrategy {
    if let Ok(mut conn) = state.db_pool().get() {
        #[derive(QueryableByName)]
        struct PrefValue {
            #[diesel(sql_type = diesel::sql_types::Text)]
            preference_value: String,
        }

        let result: Option<PrefValue> = diesel::sql_query(
            "SELECT preference_value FROM session_preferences \
             WHERE session_id = $1 AND preference_key = 'model_routing' LIMIT 1",
        )
        .bind::<diesel::sql_types::Uuid, _>(session_id)
        .get_result(&mut conn)
        .optional()
        .ok()
        .flatten();

        if let Some(pref) = result {
            match pref.preference_value.as_str() {
                "auto" => RoutingStrategy::Auto,
                "load-balanced" => RoutingStrategy::LoadBalanced,
                "fallback" => RoutingStrategy::Fallback,
                _ => RoutingStrategy::Manual,
            }
        } else {
            RoutingStrategy::Manual
        }
    } else {
        RoutingStrategy::Manual
    }
}
