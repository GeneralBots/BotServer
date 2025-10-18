use crate::shared::models::UserSession;
use crate::shared::state::AppState;
use log::{error, info};
use rhai::{Dynamic, Engine};
use std::sync::Arc;

pub fn set_kb_keyword(state: Arc<AppState>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();

    engine
        .register_custom_syntax(&["SET_KB", "$expr$"], false, move |context, inputs| {
            let kb_name = context.eval_expression_tree(&inputs[0])?;
            let kb_name_str = kb_name.to_string().trim_matches('"').to_string();

            info!(
                "SET_KB command executed: {} for user: {}",
                kb_name_str, user_clone.user_id
            );

            // Validate KB name (alphanumeric and underscores only)
            if !kb_name_str
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            {
                return Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    "KB name must contain only alphanumeric characters, underscores, and hyphens"
                        .into(),
                    rhai::Position::NONE,
                )));
            }

            if kb_name_str.is_empty() {
                return Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    "KB name cannot be empty".into(),
                    rhai::Position::NONE,
                )));
            }

            let state_for_task = Arc::clone(&state_clone);
            let user_for_task = user_clone.clone();
            let kb_name_for_task = kb_name_str.clone();

            // Spawn async task to set up KB collection
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build();

                let send_err = if let Ok(rt) = rt {
                    let result = rt.block_on(async move {
                        add_kb_to_user(
                            &state_for_task,
                            &user_for_task,
                            &kb_name_for_task,
                            false,
                            None,
                        )
                        .await
                    });
                    tx.send(result).err()
                } else {
                    tx.send(Err("failed to build tokio runtime".into())).err()
                };

                if send_err.is_some() {
                    error!("Failed to send result from thread");
                }
            });

            match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                Ok(Ok(message)) => {
                    info!("SET_KB completed: {}", message);
                    Ok(Dynamic::from(message))
                }
                Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    e.into(),
                    rhai::Position::NONE,
                ))),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "SET_KB timed out".into(),
                        rhai::Position::NONE,
                    )))
                }
                Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("SET_KB failed: {}", e).into(),
                    rhai::Position::NONE,
                ))),
            }
        })
        .unwrap();
}

pub fn add_kb_keyword(state: Arc<AppState>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();

    engine
        .register_custom_syntax(&["ADD_KB", "$expr$"], false, move |context, inputs| {
            let kb_name = context.eval_expression_tree(&inputs[0])?;
            let kb_name_str = kb_name.to_string().trim_matches('"').to_string();

            info!(
                "ADD_KB command executed: {} for user: {}",
                kb_name_str, user_clone.user_id
            );

            // Validate KB name
            if !kb_name_str
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            {
                return Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    "KB name must contain only alphanumeric characters, underscores, and hyphens"
                        .into(),
                    rhai::Position::NONE,
                )));
            }

            let state_for_task = Arc::clone(&state_clone);
            let user_for_task = user_clone.clone();
            let kb_name_for_task = kb_name_str.clone();

            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build();

                let send_err = if let Ok(rt) = rt {
                    let result = rt.block_on(async move {
                        add_kb_to_user(
                            &state_for_task,
                            &user_for_task,
                            &kb_name_for_task,
                            false,
                            None,
                        )
                        .await
                    });
                    tx.send(result).err()
                } else {
                    tx.send(Err("failed to build tokio runtime".into())).err()
                };

                if send_err.is_some() {
                    error!("Failed to send result from thread");
                }
            });

            match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                Ok(Ok(message)) => {
                    info!("ADD_KB completed: {}", message);
                    Ok(Dynamic::from(message))
                }
                Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    e.into(),
                    rhai::Position::NONE,
                ))),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "ADD_KB timed out".into(),
                        rhai::Position::NONE,
                    )))
                }
                Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("ADD_KB failed: {}", e).into(),
                    rhai::Position::NONE,
                ))),
            }
        })
        .unwrap();
}

/// Add a KB to user's active KBs (stored in user_kb_associations table)
async fn add_kb_to_user(
    _state: &AppState,
    user: &UserSession,
    kb_name: &str,
    is_website: bool,
    website_url: Option<String>,
) -> Result<String, String> {
    // TODO: Insert into user_kb_associations table using Diesel
    // For now, just log the action

    info!(
        "KB '{}' associated with user '{}' (bot: {}, is_website: {})",
        kb_name, user.user_id, user.bot_id, is_website
    );

    if is_website {
        if let Some(url) = website_url {
            info!("Website URL: {}", url);
            return Ok(format!(
                "Website KB '{}' added successfully for user",
                kb_name
            ));
        }
    }

    Ok(format!("KB '{}' added successfully for user", kb_name))
}
