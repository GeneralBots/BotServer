use botbasic_types::UserSession;
use botbasic_types::BasicRuntime;
use diesel::prelude::*;
use log::{error, trace};
use rhai::{Dynamic, Engine};
use std::sync::Arc;
use uuid::Uuid;

pub fn register_bot_memory_keywords(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
set_bot_memory_keyword(state.clone(), user.clone(), engine);
get_bot_memory_keyword(state, user, engine);
}

pub fn set_bot_memory_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();


    if let Err(e) = engine
        .register_custom_syntax(
            ["SET", "BOT", "MEMORY", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let key = context.eval_expression_tree(&inputs[0])?.to_string();
                let value = context.eval_expression_tree(&inputs[1])?.to_string();
                let state_for_spawn = Arc::clone(&state_clone);
                let user_clone_spawn = user_clone.clone();
                let key_clone = key;
                let value_clone = value;

                tokio::spawn(async move {
                    use botbasic_types::schema::bot_memories;

                    let mut conn = match state_for_spawn.db_pool().get() {
                        Ok(conn) => conn,
                        Err(e) => {
                            error!(
                                "Failed to acquire database connection for SET BOT MEMORY: {}",
                                e
                            );
                            return;
                        }
                    };

                    let bot_uuid = match Uuid::parse_str(&user_clone_spawn.bot_id.to_string()) {
                        Ok(uuid) => uuid,
                        Err(e) => {
                            log::error!("Invalid bot ID format: {}", e);
                            return;
                        }
                    };

                    let now = chrono::Utc::now();

                    let existing_memory: Option<Uuid> = bot_memories::table
                        .filter(bot_memories::bot_id.eq(bot_uuid))
                        .filter(bot_memories::key.eq(&key_clone))
                        .select(bot_memories::id)
                        .first(&mut *conn)
                        .optional()
                        .unwrap_or(None);

                    if let Some(memory_id) = existing_memory {
                        let update_result = diesel::update(
                            bot_memories::table.filter(bot_memories::id.eq(memory_id)),
                        )
                        .set((
                            bot_memories::value.eq(&value_clone),
                            bot_memories::updated_at.eq(now),
                        ))
                        .execute(&mut *conn);

                        match update_result {
                            Ok(_) => {
                                trace!(
                                    "Updated bot memory for key: {} with value length: {}",
                                    key_clone,
                                    value_clone.len()
                                );
                            }
                            Err(e) => {
                                log::error!("Failed to update bot memory: {}", e);
                            }
                        }
                    } else {
                        // `bot_memories.branch_id` is NOT NULL since
                        // `6.5.23-branch-scope-cleanup`; it is the branch that
                        // owns the bot. The nil UUID is the same fallback that
                        // migration used when backfilling existing rows.
                        let branch_id: Uuid = botbasic_types::schema::bots::table
                            .filter(botbasic_types::schema::bots::id.eq(bot_uuid))
                            .select(botbasic_types::schema::bots::branch_id)
                            .first::<Uuid>(&mut *conn)
                            .optional()
                            .unwrap_or(None)
                            .unwrap_or_else(Uuid::nil);

                        let new_memory = botbasic_types::types::BotMemory {
                            id: Uuid::new_v4(),
                            bot_id: bot_uuid,
                            key: key_clone.clone(),
                            value: value_clone.clone(),
                            created_at: now,
                            updated_at: now,
                            branch_id,
                        };

    let insert_result = diesel::sql_query(
        "INSERT INTO bot_memories (id, branch_id, bot_id, key, value, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)"
    )
    .bind::<diesel::sql_types::Uuid, _>(new_memory.id)
    .bind::<diesel::sql_types::Uuid, _>(new_memory.branch_id)
    .bind::<diesel::sql_types::Uuid, _>(new_memory.bot_id)
    .bind::<diesel::sql_types::Text, _>(&new_memory.key)
    .bind::<diesel::sql_types::Text, _>(&new_memory.value)
    .bind::<diesel::sql_types::Timestamptz, _>(new_memory.created_at)
    .bind::<diesel::sql_types::Timestamptz, _>(new_memory.updated_at)
    .execute(&mut *conn);

                        match insert_result {
                            Ok(_) => {
                                trace!(
                                    "Created new bot memory for key: {} with value length: {}",
                                    key_clone,
                                    value_clone.len()
                                );
                            }
                            Err(e) => {
                                log::error!("Failed to insert bot memory: {}", e);
                            }
                        }
                    }
                });

                Ok(Dynamic::UNIT)
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    // Also register as underscore function for tools (SET_BOT_MEMORY(key, value))
    let state_clone2 = Arc::clone(&state);
    let user_clone2 = user.clone();
    engine.register_fn("SET_BOT_MEMORY", move |key: String, value: String| {
        let state_for_spawn = Arc::clone(&state_clone2);
        let user_clone_spawn = user_clone2.clone();
        let key_clone = key;
        let value_clone = value;

        tokio::spawn(async move {
            use botbasic_types::schema::bot_memories;

            let mut conn = match state_for_spawn.db_pool().get() {
                Ok(conn) => conn,
                Err(e) => {
                    error!(
                        "Failed to acquire database connection for SET_BOT_MEMORY: {}",
                        e
                    );
                    return;
                }
            };

            let bot_uuid = match Uuid::parse_str(&user_clone_spawn.bot_id.to_string()) {
                Ok(uuid) => uuid,
                Err(e) => {
                    log::error!("Invalid bot ID format: {}", e);
                    return;
                }
            };

            let now = chrono::Utc::now();

            let existing_memory: Option<Uuid> = bot_memories::table
                .filter(bot_memories::bot_id.eq(bot_uuid))
                .filter(bot_memories::key.eq(&key_clone))
                .select(bot_memories::id)
                .first(&mut *conn)
                .optional()
                .unwrap_or(None);

            if let Some(memory_id) = existing_memory {
                let update_result = diesel::update(
                    bot_memories::table.filter(bot_memories::id.eq(memory_id)),
                )
                .set((
                    bot_memories::value.eq(&value_clone),
                    bot_memories::updated_at.eq(now),
                ))
                .execute(&mut *conn);

                match update_result {
                    Ok(_) => {
                        trace!(
                            "Updated bot memory for key: {} with value length: {}",
                            key_clone,
                            value_clone.len()
                        );
                    }
                    Err(e) => {
                        log::error!("Failed to update bot memory: {}", e);
                    }
                }
            } else {
                let insert_result = diesel::insert_into(bot_memories::table)
                    .values((
                        bot_memories::id.eq(Uuid::new_v4()),
                        bot_memories::bot_id.eq(bot_uuid),
                        bot_memories::key.eq(&key_clone),
                        bot_memories::value.eq(&value_clone),
                        bot_memories::created_at.eq(now),
                        bot_memories::updated_at.eq(now),
                    ))
                    .execute(&mut *conn);

                match insert_result {
                    Ok(_) => {
                        trace!(
                            "Created bot memory for key: {} with value length: {}",
                            key_clone,
                            value_clone.len()
                        );
                    }
                    Err(e) => {
                        log::error!("Failed to insert bot memory: {}", e);
                    }
                }
            }
        });
    });

    // Also register as lowercase function for .ast compatibility (set_bot_memory(key, value))
    let state_clone_lower = Arc::clone(&state);
    let user_clone_lower = user.clone();
    engine.register_fn("set_bot_memory", move |key: String, value: String| {
        let state_for_spawn = Arc::clone(&state_clone_lower);
        let user_clone_spawn = user_clone_lower.clone();
        let key_clone = key;
        let value_clone = value;

        tokio::spawn(async move {
            use botbasic_types::schema::bot_memories;

            let mut conn = match state_for_spawn.db_pool().get() {
                Ok(conn) => conn,
                Err(e) => {
                    error!(
                        "Failed to acquire database connection for set_bot_memory: {}",
                        e
                    );
                    return;
                }
            };

            let bot_uuid = match Uuid::parse_str(&user_clone_spawn.bot_id.to_string()) {
                Ok(uuid) => uuid,
                Err(e) => {
                    log::error!("Invalid bot ID format: {}", e);
                    return;
                }
            };

            let now = chrono::Utc::now();

            let existing_memory: Option<Uuid> = bot_memories::table
                .filter(bot_memories::bot_id.eq(bot_uuid))
                .filter(bot_memories::key.eq(&key_clone))
                .select(bot_memories::id)
                .first(&mut *conn)
                .optional()
                .unwrap_or(None);

            if let Some(memory_id) = existing_memory {
                let update_result = diesel::update(
                    bot_memories::table.filter(bot_memories::id.eq(memory_id)),
                )
                .set((
                    bot_memories::value.eq(&value_clone),
                    bot_memories::updated_at.eq(now),
                ))
                .execute(&mut *conn);

                match update_result {
                    Ok(_) => {
                        trace!(
                            "Updated bot memory for key: {} with value length: {}",
                            key_clone,
                            value_clone.len()
                        );
                    }
                    Err(e) => {
                        log::error!("Failed to update bot memory: {}", e);
                    }
                }
            } else {
                // See the sibling `SET_BOT_MEMORY` path above: `branch_id` is
                // NOT NULL and the insert must carry it.
                let branch_id: Uuid = botbasic_types::schema::bots::table
                    .filter(botbasic_types::schema::bots::id.eq(bot_uuid))
                    .select(botbasic_types::schema::bots::branch_id)
                    .first::<Uuid>(&mut *conn)
                    .optional()
                    .unwrap_or(None)
                    .unwrap_or_else(Uuid::nil);

                let new_memory = botbasic_types::types::BotMemory {
                    id: Uuid::new_v4(),
                    bot_id: bot_uuid,
                    key: key_clone.clone(),
                    value: value_clone.clone(),
                    created_at: now,
                    updated_at: now,
                    branch_id,
                };

                let insert_result = diesel::sql_query(
                    "INSERT INTO bot_memories (id, branch_id, bot_id, key, value, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)"
                )
                .bind::<diesel::sql_types::Uuid, _>(new_memory.id)
                .bind::<diesel::sql_types::Uuid, _>(new_memory.branch_id)
                .bind::<diesel::sql_types::Uuid, _>(new_memory.bot_id)
                .bind::<diesel::sql_types::Text, _>(&new_memory.key)
                .bind::<diesel::sql_types::Text, _>(&new_memory.value)
                .bind::<diesel::sql_types::Timestamptz, _>(new_memory.created_at)
                .bind::<diesel::sql_types::Timestamptz, _>(new_memory.updated_at)
                .execute(&mut *conn);

                match insert_result {
                    Ok(_) => {
                        trace!(
                            "Created bot memory for key: {} with value length: {}",
                            key_clone,
                            value_clone.len()
                        );
                    }
                    Err(e) => {
                        log::error!("Failed to insert bot memory: {}", e);
                    }
                }
            }
        });
    });

    // Also register GET_BOT_MEMORY as underscore function for tools
    let state_clone3 = Arc::clone(&state);
    let user_clone3 = user.clone();
    engine.register_fn("GET_BOT_MEMORY", move |key_param: String| -> String {
        use botbasic_types::schema::bot_memories;

        let state = Arc::clone(&state_clone3);
        let conn_result = state.db_pool().get();

        if let Ok(mut conn) = conn_result {
            let bot_uuid = user_clone3.bot_id;

            let memory_value: Option<String> = bot_memories::table
                .filter(bot_memories::bot_id.eq(bot_uuid))
                .filter(bot_memories::key.eq(&key_param))
                .select(bot_memories::value)
                .first(&mut *conn)
                .optional()
                .unwrap_or(None);

            memory_value.unwrap_or_default()
        } else {
            String::new()
        }
    });

    // Also register as lowercase function for .ast compatibility (get_bot_memory(key))
    let state_clone_lower_get = Arc::clone(&state);
    let user_clone_lower_get = user.clone();
    engine.register_fn("get_bot_memory", move |key_param: String| -> String {
        use botbasic_types::schema::bot_memories;

        let state = Arc::clone(&state_clone_lower_get);
        let conn_result = state.db_pool().get();

        if let Ok(mut conn) = conn_result {
            let bot_uuid = user_clone_lower_get.bot_id;

            let memory_value: Option<String> = bot_memories::table
                .filter(bot_memories::bot_id.eq(bot_uuid))
                .filter(bot_memories::key.eq(&key_param))
                .select(bot_memories::value)
                .first(&mut *conn)
                .optional()
                .unwrap_or(None);

            memory_value.unwrap_or_default()
        } else {
            String::new()
        }
    });
}

pub fn get_bot_memory_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;


    engine.register_fn("GET BOT MEMORY", move |key_param: String| -> String {
        use botbasic_types::schema::bot_memories;

        let state = Arc::clone(&state_clone);
        let conn_result = state.db_pool().get();

        if let Ok(mut conn) = conn_result {
            let bot_uuid = user_clone.bot_id;

            let memory_value: Option<String> = bot_memories::table
                .filter(bot_memories::bot_id.eq(bot_uuid))
                .filter(bot_memories::key.eq(&key_param))
                .select(bot_memories::value)
                .first(&mut *conn)
                .optional()
                .unwrap_or(None);

            memory_value.unwrap_or_default()
        } else {
            String::new()
        }
    });
}
