use crate::shared::state::AppState;
use crate::{channels::ChannelAdapter, shared::models::UserSession};
use log::info;
use rhai::{Dynamic, Engine, EvalAltResult};

pub fn hear_keyword(state: &AppState, user: UserSession, engine: &mut Engine) {
    let session_id = user.id;
    let cache = state.redis_client.clone();

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

            let cache_clone = cache.clone();
            let session_id_clone = session_id;
            let var_name_clone = variable_name.clone();

            tokio::spawn(async move {
                log::debug!(
                    "HEAR: Starting async task for session {} and variable '{}'",
                    session_id_clone,
                    var_name_clone
                );

                if let Some(cache_client) = &cache_clone {
                    let mut conn = match cache_client.get_multiplexed_async_connection().await {
                        Ok(conn) => conn,
                        Err(e) => {
                            log::error!("Failed to connect to cache: {}", e);
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

pub fn talk_keyword(state: &AppState, user: UserSession, engine: &mut Engine) {
    use crate::shared::models::BotResponse;

    let state_clone = state.clone();
    let user_clone = user.clone();

    engine
        .register_custom_syntax(&["TALK", "$expr$"], true, move |context, inputs| {
            let message = context.eval_expression_tree(&inputs[0])?.to_string();

            info!("TALK command executed: {}", message);

            let response = BotResponse {
                bot_id: "default_bot".to_string(),
                user_id: user_clone.user_id.to_string(),
                session_id: user_clone.id.to_string(),
                channel: "basic".to_string(),
                content: message,
                message_type: 1,
                stream_token: None,
                is_complete: true,
            };

            let state_for_spawn = state_clone.clone();
            tokio::spawn(async move {
                if let Err(e) = state_for_spawn.web_adapter.send_message(response).await {
                    log::error!("Failed to send TALK message: {}", e);
                }
            });

            Ok(Dynamic::UNIT)
        })
        .unwrap();
}

pub fn set_context_keyword(state: &AppState, user: UserSession, engine: &mut Engine) {
    let cache = state.redis_client.clone();

    engine
        .register_custom_syntax(
            &["SET", "CONTEXT", "$expr$"],
            true,
            move |context, inputs| {
                let context_value = context.eval_expression_tree(&inputs[0])?.to_string();

                info!("SET CONTEXT command executed: {}", context_value);

                let redis_key = format!("context:{}:{}", user.user_id, user.id);

                let cache_clone = cache.clone();

                tokio::spawn(async move {
                    if let Some(cache_client) = &cache_clone {
                        let mut conn = match cache_client.get_multiplexed_async_connection().await {
                            Ok(conn) => conn,
                            Err(e) => {
                                log::error!("Failed to connect to cache: {}", e);
                                return;
                            }
                        };

                        let _: Result<(), _> = redis::cmd("SET")
                            .arg(&redis_key)
                            .arg(&context_value)
                            .query_async(&mut conn)
                            .await;
                    }
                });

                Ok(Dynamic::UNIT)
            },
        )
        .unwrap();
}
