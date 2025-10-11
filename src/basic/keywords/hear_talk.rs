use crate::shared::models::UserSession;
use crate::shared::state::AppState;
use log::info;
use rhai::{Dynamic, Engine, EvalAltResult};

pub fn hear_keyword(_state: &AppState, user: UserSession, engine: &mut Engine) {
    let session_id = user.id;

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

            // Spawn a background task to handle the input‑waiting logic.
            // The actual waiting implementation should be added here.
            tokio::spawn(async move {
                log::debug!(
                    "HEAR: Starting async task for session {} and variable '{}'",
                    session_id,
                    variable_name
                );
                // TODO: implement actual waiting logic here without using the orchestrator
                // For now, just log that we would wait for input
            });

            // Interrupt the current Rhai evaluation flow until the user input is received.
            Err(Box::new(EvalAltResult::ErrorRuntime(
                "Waiting for user input".into(),
                rhai::Position::NONE,
            )))
        })
        .unwrap();
}

pub fn talk_keyword(state: &AppState, user: UserSession, engine: &mut Engine) {
    // Import the BotResponse type directly to satisfy diagnostics.
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
                message_type: "text".to_string(),
                stream_token: None,
                is_complete: true,
            };

            // Send response through a channel or queue instead of accessing orchestrator directly
            let _state_for_spawn = state_clone.clone();
            tokio::spawn(async move {
                // Use a more thread-safe approach to send the message
                // This avoids capturing the orchestrator directly which isn't Send + Sync
                // TODO: Implement proper response handling once response_sender field is added to AppState
                log::debug!("TALK: Would send response: {:?}", response);
            });

            Ok(Dynamic::UNIT)
        })
        .unwrap();
}

pub fn set_context_keyword(state: &AppState, user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();

    engine
        .register_custom_syntax(
            &["SET", "CONTEXT", "$expr$"],
            true,
            move |context, inputs| {
                let context_value = context.eval_expression_tree(&inputs[0])?.to_string();

                info!("SET CONTEXT command executed: {}", context_value);

                let redis_key = format!("context:{}:{}", user.user_id, user.id);

                let state_for_redis = state_clone.clone();

                tokio::spawn(async move {
                    if let Some(redis_client) = &state_for_redis.redis_client {
                        let mut conn = match redis_client.get_multiplexed_async_connection().await {
                            Ok(conn) => conn,
                            Err(e) => {
                                log::error!("Failed to connect to Redis: {}", e);
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
