use crate::shared::models::UserSession;
use crate::shared::state::AppState;
use diesel::prelude::*;
use log::{error, info};
use rhai::{Dynamic, Engine};
use std::sync::Arc;
use uuid::Uuid;

pub fn set_bot_memory_keyword(state: Arc<AppState>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();

    engine
        .register_custom_syntax(
            &["SET_BOT_MEMORY", "$expr$", "$expr$"],
            true,
            move |context, inputs| {
                let key = context.eval_expression_tree(&inputs[0])?.to_string();
                let value = context.eval_expression_tree(&inputs[1])?.to_string();

                let state_for_spawn = Arc::clone(&state_clone);
                let user_clone_spawn = user_clone.clone();
                let key_clone = key.clone();
                let value_clone = value.clone();

                tokio::spawn(async move {
                    use crate::shared::models::bot_memories;

                    let mut conn = match state_for_spawn.conn.lock() {
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
                            error!("Invalid bot ID format: {}", e);
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
                                info!(
                                    "Updated bot memory for key: {} with value length: {}",
                                    key_clone,
                                    value_clone.len()
                                );
                            }
                            Err(e) => {
                                error!("Failed to update bot memory: {}", e);
                            }
                        }
                    } else {
                        let new_memory = crate::shared::models::BotMemory {
                            id: Uuid::new_v4(),
                            bot_id: bot_uuid,
                            key: key_clone.clone(),
                            value: value_clone.clone(),
                            created_at: now,
                            updated_at: now,
                        };

                        let insert_result = diesel::insert_into(bot_memories::table)
                            .values(&new_memory)
                            .execute(&mut *conn);

                        match insert_result {
                            Ok(_) => {
                                info!(
                                    "Created new bot memory for key: {} with value length: {}",
                                    key_clone,
                                    value_clone.len()
                                );
                            }
                            Err(e) => {
                                error!("Failed to insert bot memory: {}", e);
                            }
                        }
                    }
                });

                Ok(Dynamic::UNIT)
            },
        )
        .unwrap();
}

pub fn get_bot_memory_keyword(state: Arc<AppState>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();

    engine.register_fn("GET_BOT_MEMORY", move |key_param: String| -> String {
        use crate::shared::models::bot_memories;

        let state = Arc::clone(&state_clone);

        let conn_result = state.conn.lock();
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
