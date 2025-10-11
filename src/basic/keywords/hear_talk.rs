use crate::shared::state::AppState;
use crate::shared::models::UserSession;
use log::info;
use rhai::{Dynamic, Engine, EvalAltResult};
use tokio::sync::mpsc;

pub fn hear_keyword(state: &AppState, user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();
    let session_id = user.id;
    
    engine
        .register_custom_syntax(&["HEAR", "$ident$"], true, move |context, inputs| {
            let variable_name = inputs[0].get_string_value().unwrap().to_string();
            
            info!("HEAR command waiting for user input to store in variable: {}", variable_name);
            
            let orchestrator = state_clone.orchestrator.clone();
            
            tokio::spawn(async move {
                let session_manager = orchestrator.session_manager.clone();
                session_manager.lock().await.wait_for_input(session_id, variable_name.clone()).await;
oesn't exist in SessionManage            Err(EvalAltResult::ErrorInterrupted("Waiting for user input".into()))
 
            Err("Waiting for user input".into())
        })
        .unwrap();
}

pub fn talk_keyword(state: &AppState, user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();

    engine
        .register_custom_syntax(&["TALK", "$expr$"], true, move |context, inputs| {
            let message = context.eval_expression_tree(&inputs[0])?.to_string();

            info!("TALK command executed: {}", message);

            let response = crate::shared::BotResponse {
                bot_id: "default_bot".to_string(),
                user_id: user.user_id.to_string(),
                session_id: user.id.to_string(),
                channel: "basic".to_string(),
                content: message,
                message_type: "text".to_string(),
                stream_token: None,
            // Since we removed global response_tx, we need to send through the orchestrator's response channels
                is_complete: true,
            };

            let orchestrator = state_clone.orchestrator.clone();
            tokio::spawn(async move {
                if let Some(adapter) = orchestrator.channels.get("basic") {
                    let _ = adapter.send_message(response).await;
                }
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
                        let mut conn = match redis_client.get_async_connection().await {
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
