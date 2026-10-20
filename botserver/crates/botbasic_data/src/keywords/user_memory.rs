use botbasic_types::UserSession;
use botbasic_types::BasicRuntime;
use diesel::prelude::*;
use log::trace;
use rhai::{Dynamic, Engine};
use std::sync::Arc;
use uuid::Uuid;

pub fn register_user_memory_keywords(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    set_user_memory_keyword(Arc::clone(&state), user.clone(), engine);
    get_user_memory_keyword(Arc::clone(&state), user.clone(), engine);
    remember_user_fact_keyword(Arc::clone(&state), user.clone(), engine);
    get_user_facts_keyword(Arc::clone(&state), user.clone(), engine);
    clear_user_memory_keyword(state, user, engine);
}

pub fn set_user_memory_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_id = Uuid::parse_str(&user.user_id).unwrap_or(Uuid::nil());

    if let Err(e) = engine
        .register_custom_syntax(
            ["SET", "USER", "MEMORY", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let key = context.eval_expression_tree(&inputs[0])?.to_string();
                let value = context.eval_expression_tree(&inputs[1])?.to_string();
                let state_for_spawn = Arc::clone(&state_clone);
                let key_clone = key;
                let value_clone = value;

                tokio::spawn(async move {
            if let Err(e) = set_user_memory(
                state_for_spawn,
                        user_id,
                        &key_clone,
                        &value_clone,
                        "preference",
                    ) {
                        log::error!("Failed to set user memory: {e}");
                    } else {
                        trace!(
                            "Set user memory for key: {key_clone} with value length: {}",
                            value_clone.len()
                        );
                    }
                });

                Ok(Dynamic::UNIT)
            },
        )
    {
        log::error!("Failed to register SET USER MEMORY syntax failed: {e}");
    }
}

pub fn get_user_memory_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_id = Uuid::parse_str(&user.user_id).unwrap_or(Uuid::nil());

    engine.register_fn("GET USER MEMORY", move |key_param: String| -> String {
        let state = Arc::clone(&state_clone);
        let conn_result = state.db_pool().get();

        if let Ok(mut conn) = conn_result {
            get_user_memory_sync(&mut conn, user_id, &key_param).unwrap_or_default()
        } else {
            String::new()
        }
    });
}

pub fn remember_user_fact_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_id = Uuid::parse_str(&user.user_id).unwrap_or(Uuid::nil());

    if let Err(e) = engine
        .register_custom_syntax(
            ["REMEMBER", "USER", "FACT", "$expr$"],
            false,
            move |context, inputs| {
                let fact = context.eval_expression_tree(&inputs[0])?.to_string();
                let state_for_spawn = Arc::clone(&state_clone);
                let fact_clone = fact;

                tokio::spawn(async move {
                    if let Err(e) = add_user_fact(state_for_spawn, user_id, &fact_clone) {
                        log::error!("Failed to remember user fact: {e}");
                    } else {
                        trace!("Remembered user fact: {fact_clone}");
                    }
                });

                Ok(Dynamic::UNIT)
            },
        )
    {
        log::error!("Failed to register REMEMBER USER FACT syntax failed: {e}");
    }
}

pub fn get_user_facts_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_id = Uuid::parse_str(&user.user_id).unwrap_or(Uuid::nil());

    engine.register_fn("GET USER FACTS", move || -> rhai::Array {
        let state = Arc::clone(&state_clone);
        let conn_result = state.db_pool().get();

        if let Ok(mut conn) = conn_result {
            get_user_facts_sync(&mut conn, user_id)
                .unwrap_or_default()
                .into_iter()
                .map(Dynamic::from)
                .collect()
        } else {
            rhai::Array::new()
        }
    });
}

pub fn clear_user_memory_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_id = Uuid::parse_str(&user.user_id).unwrap_or(Uuid::nil());

    if let Err(e) = engine
        .register_custom_syntax(
            ["CLEAR", "USER", "MEMORY"],
            false,
            move |_context, _inputs| {
                let state_for_spawn = Arc::clone(&state_clone);

                tokio::spawn(async move {
                    if let Err(e) = clear_user_memory(state_for_spawn, user_id) {
                        log::error!("Failed to clear user memory: {e}");
                    } else {
                        trace!("Cleared all user memory for user: {user_id}");
                    }
                });

                Ok(Dynamic::UNIT)
            },
        )
    {
        log::error!("Failed to register CLEAR USER MEMORY syntax failed: {e}");
    }
}

fn set_user_memory(
    state: Arc<dyn BasicRuntime>,
    user_id: Uuid,
    key: &str,
    value: &str,
    memory_type: &str,
) -> Result<(), String> {
    let mut conn = state
        .db_pool()
        .get()
        .map_err(|e| format!("Failed to acquire database connection: {e}"))?;

    let now = chrono::Utc::now();
    let new_id = Uuid::new_v4();

    diesel::sql_query(
        "INSERT INTO user_memories (id, user_id, key, value, memory_type, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (user_id, key) DO UPDATE SET \
         value = EXCLUDED.value, \
         memory_type = EXCLUDED.memory_type, \
         updated_at = EXCLUDED.updated_at",
    )
    .bind::<diesel::sql_types::Uuid, _>(new_id)
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .bind::<diesel::sql_types::Text, _>(key)
    .bind::<diesel::sql_types::Text, _>(value)
    .bind::<diesel::sql_types::Text, _>(memory_type)
    .bind::<diesel::sql_types::Timestamptz, _>(now)
    .bind::<diesel::sql_types::Timestamptz, _>(now)
    .execute(&mut conn)
    .map_err(|e| format!("Failed to set user memory: {e}"))?;

    Ok(())
}

fn get_user_memory_sync(
    conn: &mut diesel::PgConnection,
    user_id: Uuid,
    key: &str,
) -> Result<String, String> {
    #[derive(QueryableByName)]
    struct MemoryValue {
        #[diesel(sql_type = diesel::sql_types::Text)]
        value: String,
    }

    let result: Option<MemoryValue> = diesel::sql_query(
        "SELECT value FROM user_memories WHERE user_id = $1 AND key = $2 LIMIT 1",
    )
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .bind::<diesel::sql_types::Text, _>(key)
    .get_result(conn)
    .optional()
    .map_err(|e| format!("Failed to get user memory: {e}"))?;

    Ok(result.map(|r| r.value).unwrap_or_default())
}

fn add_user_fact(state: Arc<dyn BasicRuntime>, user_id: Uuid, fact: &str) -> Result<(), String> {
    let mut conn = state
        .db_pool()
        .get()
        .map_err(|e| format!("Failed to acquire database connection: {e}"))?;

    let now = chrono::Utc::now();
    let new_id = Uuid::new_v4();
    let fact_key = format!("fact_{}", Uuid::new_v4());

    diesel::sql_query(
        "INSERT INTO user_memories (id, user_id, key, value, memory_type, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, 'fact', $5, $6)",
    )
    .bind::<diesel::sql_types::Uuid, _>(new_id)
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .bind::<diesel::sql_types::Text, _>(&fact_key)
    .bind::<diesel::sql_types::Text, _>(fact)
    .bind::<diesel::sql_types::Timestamptz, _>(now)
    .bind::<diesel::sql_types::Timestamptz, _>(now)
    .execute(&mut conn)
    .map_err(|e| format!("Failed to add user fact: {e}"))?;

    Ok(())
}

fn get_user_facts_sync(
    conn: &mut diesel::PgConnection,
    user_id: Uuid,
) -> Result<Vec<String>, String> {
    #[derive(QueryableByName)]
    struct FactValue {
        #[diesel(sql_type = diesel::sql_types::Text)]
        value: String,
    }

    let results: Vec<FactValue> = diesel::sql_query(
        "SELECT value FROM user_memories WHERE user_id = $1 AND memory_type = 'fact' ORDER BY created_at DESC",
    )
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .load(conn)
    .map_err(|e| format!("Failed to get user facts: {e}"))?;

    Ok(results.into_iter().map(|r| r.value).collect())
}

fn clear_user_memory(state: Arc<dyn BasicRuntime>, user_id: Uuid) -> Result<(), String> {
    let mut conn = state
        .db_pool()
        .get()
        .map_err(|e| format!("Failed to acquire database connection: {e}"))?;

    diesel::sql_query("DELETE FROM user_memories WHERE user_id = $1")
        .bind::<diesel::sql_types::Uuid, _>(user_id)
        .execute(&mut conn)
        .map_err(|e| format!("Failed to clear user memory: {e}"))?;

    Ok(())
}
