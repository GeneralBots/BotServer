use crate::basic::keywords::add_tool::remove_session_tool;
use crate::shared::models::UserSession;
use crate::shared::state::AppState;
use log::{error, info};
use rhai::{Dynamic, Engine};
use std::sync::Arc;

pub fn remove_tool_keyword(state: Arc<AppState>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();

    engine
        .register_custom_syntax(&["REMOVE_TOOL", "$expr$"], false, move |context, inputs| {
            let tool_path = context.eval_expression_tree(&inputs[0])?;
            let tool_path_str = tool_path.to_string().trim_matches('"').to_string();

            info!(
                "REMOVE_TOOL command executed: {} for session: {}",
                tool_path_str, user_clone.id
            );

            // Extract tool name from path (e.g., "enrollment.bas" -> "enrollment")
            let tool_name = tool_path_str
                .strip_prefix(".gbdialog/")
                .unwrap_or(&tool_path_str)
                .strip_suffix(".bas")
                .unwrap_or(&tool_path_str)
                .to_string();

            // Validate tool name
            if tool_name.is_empty() {
                return Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    "Invalid tool name".into(),
                    rhai::Position::NONE,
                )));
            }

            let state_for_task = Arc::clone(&state_clone);
            let user_for_task = user_clone.clone();
            let tool_name_for_task = tool_name.clone();

            // Spawn async task to remove tool association from session
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build();

                let send_err = if let Ok(rt) = rt {
                    let result = rt.block_on(async move {
                        disassociate_tool_from_session(
                            &state_for_task,
                            &user_for_task,
                            &tool_name_for_task,
                        )
                        .await
                    });
                    tx.send(result).err()
                } else {
                    tx.send(Err("Failed to build tokio runtime".to_string()))
                        .err()
                };

                if send_err.is_some() {
                    error!("Failed to send result from thread");
                }
            });

            match rx.recv_timeout(std::time::Duration::from_secs(10)) {
                Ok(Ok(message)) => {
                    info!("REMOVE_TOOL completed: {}", message);
                    Ok(Dynamic::from(message))
                }
                Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    e.into(),
                    rhai::Position::NONE,
                ))),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "REMOVE_TOOL timed out".into(),
                        rhai::Position::NONE,
                    )))
                }
                Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("REMOVE_TOOL failed: {}", e).into(),
                    rhai::Position::NONE,
                ))),
            }
        })
        .unwrap();
}

/// Remove a tool association from the current session
async fn disassociate_tool_from_session(
    state: &AppState,
    user: &UserSession,
    tool_name: &str,
) -> Result<String, String> {
    let mut conn = state.conn.lock().map_err(|e| {
        error!("Failed to acquire database lock: {}", e);
        format!("Database connection error: {}", e)
    })?;

    // Remove the tool association
    let delete_result = remove_session_tool(&mut *conn, &user.id, tool_name);

    match delete_result {
        Ok(rows_affected) => {
            if rows_affected > 0 {
                info!(
                    "Tool '{}' removed from session '{}' (user: {}, bot: {})",
                    tool_name, user.id, user.user_id, user.bot_id
                );
                Ok(format!(
                    "Tool '{}' has been removed from this conversation",
                    tool_name
                ))
            } else {
                info!(
                    "Tool '{}' was not associated with session '{}'",
                    tool_name, user.id
                );
                Ok(format!(
                    "Tool '{}' was not active in this conversation",
                    tool_name
                ))
            }
        }
        Err(e) => {
            error!(
                "Failed to remove tool '{}' from session '{}': {}",
                tool_name, user.id, e
            );
            Err(format!("Failed to remove tool from session: {}", e))
        }
    }
}
