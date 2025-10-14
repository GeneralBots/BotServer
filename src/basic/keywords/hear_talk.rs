use crate::shared::models::{BotResponse, UserSession};
use crate::shared::state::AppState;
use log::{debug, error, info};
use rhai::{Dynamic, Engine, EvalAltResult};
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
            let message = context.eval_expression_tree(&inputs[0])?.to_string();

            info!("TALK command executed: {}", message);

            let state_for_spawn = Arc::clone(&state_clone);
            let user_clone_spawn = user_clone.clone();
            let message_clone = message.clone();

            tokio::spawn(async move {
                debug!("TALK: Sending message via WebSocket: {}", message_clone);

                let bot_id =
                    std::env::var("BOT_GUID").unwrap_or_else(|_| "default_bot".to_string());

                let response = BotResponse {
                    bot_id: bot_id,
                    user_id: user_clone_spawn.user_id.to_string(),
                    session_id: user_clone_spawn.id.to_string(),
                    channel: "web".to_string(),
                    content: message_clone,
                    message_type: 1,
                    stream_token: None,
                    is_complete: true,
                };

                let response_channels = state_for_spawn.response_channels.lock().await;
                if let Some(tx) = response_channels.get(&user_clone_spawn.id.to_string()) {
                    if let Err(e) = tx.send(response).await {
                        error!("Failed to send TALK message via WebSocket: {}", e);
                    } else {
                        debug!("TALK message sent successfully via WebSocket");
                    }
                } else {
                    debug!(
                        "No WebSocket connection found for session {}, sending via web adapter",
                        user_clone_spawn.id
                    );

                    if let Err(e) = state_for_spawn
                        .web_adapter
                        .send_message_to_session(&user_clone_spawn.id.to_string(), response)
                        .await
                    {
                        error!("Failed to send TALK message via web adapter: {}", e);
                    } else {
                        debug!("TALK message sent successfully via web adapter");
                    }
                }
            });

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
            let context_value = context.eval_expression_tree(&inputs[0])?.to_string();

            info!("SET CONTEXT command executed: {}", context_value);

            let redis_key = format!("context:{}:{}", user.user_id, user.id);

            let cache_clone = cache.clone();

            if let Some(cache_client) = &cache_clone {
                let mut conn = match futures::executor::block_on(
                    cache_client.get_multiplexed_async_connection(),
                ) {
                    Ok(conn) => conn,
                    Err(e) => {
                        error!("Failed to connect to cache: {}", e);
                        return Ok(Dynamic::UNIT);
                    }
                };

                let _: Result<(), _> = futures::executor::block_on(
                    redis::cmd("SET")
                        .arg(&redis_key)
                        .arg(&context_value)
                        .query_async(&mut conn),
                );
            }

            Ok(Dynamic::UNIT)
        })
        .unwrap();
}
