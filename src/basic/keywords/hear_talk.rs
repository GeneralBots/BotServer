use crate::shared::models::{BotResponse, UserSession};
use crate::shared::state::AppState;
use log::{debug, error, info};
use rhai::{Dynamic, Engine, EvalAltResult};
use std::env;
use std::sync::Arc;
use uuid::Uuid;

pub fn hear_keyword(state: Arc<AppState>, user: UserSession, engine: &mut Engine) {
    let session_id = user.id;
    let state_clone = Arc::clone(&state);

    engine
        .register_custom_syntax(&["HEAR", "$ident$"], true, move |_context, inputs| {
            let variable_name = inputs[0]
                .get_string_value()
                .expect("Expected identifier as string")
                .to_string();

            info!(
                "HEAR command waiting for user input to store in variable: {}",
                variable_name
            );

            let state_for_spawn = Arc::clone(&state_clone);
            let session_id_clone = session_id;
            let var_name_clone = variable_name.clone();

            tokio::spawn(async move {
                debug!(
                    "HEAR: Setting session {} to wait for input for variable '{}'",
                    session_id_clone, var_name_clone
                );

                let mut session_manager = state_for_spawn.session_manager.lock().await;
                session_manager.mark_waiting(session_id_clone);

                if let Some(redis_client) = &state_for_spawn.redis_client {
                    let mut conn = match redis_client.get_multiplexed_async_connection().await {
                        Ok(conn) => conn,
                        Err(e) => {
                            error!("Failed to connect to cache: {}", e);
                            return;
                        }
                    };

                    let key = format!("hear:{}:{}", session_id_clone, var_name_clone);
                    let _: Result<(), _> = redis::cmd("SET")
                        .arg(&key)
                        .arg("waiting")
                        .query_async(&mut conn)
                        .await;
                }
            });

            Err(Box::new(EvalAltResult::ErrorRuntime(
                "Waiting for user input".into(),
                rhai::Position::NONE,
            )))
        })
        .unwrap();
}

pub fn talk_keyword(state: Arc<AppState>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();

    engine
        .register_custom_syntax(&["TALK", "$expr$"], true, move |context, inputs| {
            // Evaluate the expression that produces the message text.
            let message = context.eval_expression_tree(&inputs[0])?.to_string();
            info!("TALK command executed: {}", message);
            debug!("TALK: Sending message: {}", message);

            // Build the bot response that will be sent back to the client.
            let bot_id = env::var("BOT_GUID").unwrap_or_else(|_| "default_bot".to_string());
            let response = BotResponse {
                bot_id,
                user_id: user_clone.user_id.to_string(),
                session_id: user_clone.id.to_string(),
                channel: "web".to_string(),
                content: message,
                message_type: 1,
                stream_token: None,
                is_complete: true,
            };

            let user_id = user_clone.id.to_string();

            // Try to acquire the lock on the response_channels map. The map is protected
            // by an async `tokio::sync::Mutex`, so we use `try_lock` to avoid awaiting
            // inside this non‑async closure.
            match state_clone.response_channels.try_lock() {
                Ok(mut response_channels) => {
                    if let Some(tx) = response_channels.get(&user_id) {
                        // Use `try_send` to avoid blocking the runtime.
                        if let Err(e) = tx.try_send(response.clone()) {
                            error!("Failed to send TALK message via WebSocket: {}", e);
                        } else {
                            debug!("TALK message sent successfully via WebSocket");
                        }
                    } else {
                        debug!(
                            "No WebSocket connection found for session {}, sending via web adapter",
                            user_id
                        );
                        // The web adapter method is async (`send_message_to_session`), so we
                        // spawn a detached task to perform the send without blocking.
                        let web_adapter = Arc::clone(&state_clone.web_adapter);
                        let resp_clone = response.clone();
                        let sess_id = user_id.clone();
                        tokio::spawn(async move {
                            if let Err(e) = web_adapter
                                .send_message_to_session(&sess_id, resp_clone)
                                .await
                            {
                                error!("Failed to send TALK message via web adapter: {}", e);
                            } else {
                                debug!("TALK message sent successfully via web adapter");
                            }
                        });
                    }
                }
                Err(_) => {
                    error!("Failed to acquire lock on response_channels for TALK command");
                }
            }

            Ok(Dynamic::UNIT)
        })
        .unwrap();
}

pub fn set_user_keyword(state: Arc<AppState>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();
    engine
        .register_custom_syntax(&["SET_USER", "$expr$"], true, move |context, inputs| {
            let user_id_str = context.eval_expression_tree(&inputs[0])?.to_string();

            info!("SET USER command executed with ID: {}", user_id_str);

            match Uuid::parse_str(&user_id_str) {
                Ok(user_id) => {
                    debug!("Successfully parsed user UUID: {}", user_id);

                    let state_for_spawn = Arc::clone(&state_clone);
                    let user_clone_spawn = user_clone.clone();

                    let mut session_manager =
                        futures::executor::block_on(state_for_spawn.session_manager.lock());

                    if let Err(e) = session_manager.update_user_id(user_clone_spawn.id, user_id) {
                        error!("Failed to update user ID in session: {}", e);
                    } else {
                        info!(
                            "Updated session {} to user ID: {}",
                            user_clone_spawn.id, user_id
                        );
                    }
                }
                Err(e) => {
                    debug!("Invalid UUID format for SET USER: {}", e);
                }
            }

            Ok(Dynamic::UNIT)
        })
        .unwrap();
}
pub fn set_context_keyword(state: &AppState, user: UserSession, engine: &mut Engine) {
    let cache = state.redis_client.clone();

    engine
        .register_custom_syntax(&["SET_CONTEXT", "$expr$"], true, move |context, inputs| {
            // Evaluate the expression that should be stored in the context.
            let context_value = context.eval_expression_tree(&inputs[0])?.to_string();

            info!("SET CONTEXT command executed: {}", context_value);
            // Build the Redis key using the user ID and the session ID.
            let redis_key = format!("context:{}:{}", user.user_id, user.id);
            log::trace!(
                target: "app::set_context",
                "Constructed Redis key: {} for user {} and session {}",
                redis_key,
                user.user_id,
                user.id
            );

            // If a Redis client is configured, perform the SET operation in a background task.
            if let Some(cache_client) = &cache {
                log::trace!("Redis client is available, preparing to set context value");
                // Clone the values we need inside the async block.
                let cache_client = cache_client.clone();
                let redis_key = redis_key.clone();
                let context_value = context_value.clone();
                log::trace!(
                    "Cloned cache_client, redis_key ({}) and context_value (len={}) for async task",
                    redis_key,
                    context_value.len()
                );

                // Spawn a task so we don't need an async closure here.
                tokio::spawn(async move {
                    log::trace!("Async task started for SET_CONTEXT operation");
                    // Acquire an async Redis connection.
                    let mut conn = match cache_client.get_multiplexed_async_connection().await {
                        Ok(conn) => {
                            log::trace!("Successfully acquired async Redis connection");
                            conn
                        }
                        Err(e) => {
                            error!("Failed to connect to cache: {}", e);
                            log::trace!("Aborting SET_CONTEXT task due to connection error");
                            return;
                        }
                    };

                    // Perform the SET command.
                    log::trace!(
                        "Executing Redis SET command with key: {} and value length: {}",
                        redis_key,
                        context_value.len()
                    );
                    let result: Result<(), redis::RedisError> = redis::cmd("SET")
                        .arg(&redis_key)
                        .arg(&context_value)
                        .query_async(&mut conn)
                        .await;

                    match result {
                        Ok(_) => {
                            log::trace!("Successfully set context in Redis for key {}", redis_key);
                        }
                        Err(e) => {
                            error!("Failed to set cache value: {}", e);
                            log::trace!("SET_CONTEXT Redis SET command failed");
                        }
                    }
                });
            } else {
                log::trace!("No Redis client configured; SET_CONTEXT will not persist to cache");
            }

            Ok(Dynamic::UNIT)
        })
        .unwrap();
}
