use crate::basic::keywords::add_tool::clear_session_tools;
use crate::shared::models::UserSession;
use crate::shared::state::AppState;
use log::{error, info};
use rhai::{Dynamic, Engine};
use std::sync::Arc;

pub fn clear_tools_keyword(state: Arc<AppState>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();

    engine
        .register_custom_syntax(&["CLEAR_TOOLS"], false, move |_context, _inputs| {
            info!(
                "CLEAR_TOOLS command executed for session: {}",
                user_clone.id
            );

            let state_for_task = Arc::clone(&state_clone);
            let user_for_task = user_clone.clone();

            // Spawn async task to clear all tool associations from session
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build();

                let send_err = if let Ok(rt) = rt {
                    let result = rt.block_on(async move {
                        clear_all_tools_from_session(&state_for_task, &user_for_task).await
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
                    info!("CLEAR_TOOLS completed: {}", message);
                    Ok(Dynamic::from(message))
                }
                Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    e.into(),
                    rhai::Position::NONE,
                ))),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "CLEAR_TOOLS timed out".into(),
                        rhai::Position::NONE,
                    )))
                }
                Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("CLEAR_TOOLS failed: {}", e).into(),
                    rhai::Position::NONE,
                ))),
            }
        })
        .unwrap();
}

/// Clear all tool associations from the current session
async fn clear_all_tools_from_session(
    state: &AppState,
    user: &UserSession,
) -> Result<String, String> {
    let mut conn = state.conn.lock().map_err(|e| {
        error!("Failed to acquire database lock: {}", e);
        format!("Database connection error: {}", e)
    })?;

    // Clear all tool associations for this session
    let delete_result = clear_session_tools(&mut *conn, &user.id);

    match delete_result {
        Ok(rows_affected) => {
            if rows_affected > 0 {
                info!(
                    "Cleared {} tool(s) from session '{}' (user: {}, bot: {})",
                    rows_affected, user.id, user.user_id, user.bot_id
                );
                Ok(format!(
                    "All {} tool(s) have been removed from this conversation",
                    rows_affected
                ))
            } else {
                info!("No tools were associated with session '{}'", user.id);
                Ok("No tools were active in this conversation".to_string())
            }
        }
        Err(e) => {
            error!("Failed to clear tools from session '{}': {}", user.id, e);
            Err(format!("Failed to clear tools from session: {}", e))
        }
    }
}
