use crate::basic::keywords::add_tool::get_session_tools;
use crate::shared::models::UserSession;
use crate::shared::state::AppState;
use log::{error, info};
use rhai::{Dynamic, Engine};
use std::sync::Arc;

pub fn list_tools_keyword(state: Arc<AppState>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();

    engine
        .register_custom_syntax(&["LIST_TOOLS"], false, move |_context, _inputs| {
            info!("LIST_TOOLS command executed for session: {}", user_clone.id);

            let state_for_task = Arc::clone(&state_clone);
            let user_for_task = user_clone.clone();

            // Spawn async task to list all tool associations from session
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build();

                let send_err = if let Ok(rt) = rt {
                    let result = rt.block_on(async move {
                        list_session_tools(&state_for_task, &user_for_task).await
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
                    info!("LIST_TOOLS completed: {}", message);
                    Ok(Dynamic::from(message))
                }
                Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    e.into(),
                    rhai::Position::NONE,
                ))),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "LIST_TOOLS timed out".into(),
                        rhai::Position::NONE,
                    )))
                }
                Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("LIST_TOOLS failed: {}", e).into(),
                    rhai::Position::NONE,
                ))),
            }
        })
        .unwrap();
}

/// List all tools associated with the current session
async fn list_session_tools(state: &AppState, user: &UserSession) -> Result<String, String> {
    let mut conn = state.conn.lock().map_err(|e| {
        error!("Failed to acquire database lock: {}", e);
        format!("Database connection error: {}", e)
    })?;

    // Get all tool associations for this session
    match get_session_tools(&mut *conn, &user.id) {
        Ok(tools) => {
            if tools.is_empty() {
                info!("No tools associated with session '{}'", user.id);
                Ok("No tools are currently active in this conversation".to_string())
            } else {
                info!(
                    "Found {} tool(s) for session '{}' (user: {}, bot: {})",
                    tools.len(),
                    user.id,
                    user.user_id,
                    user.bot_id
                );

                let tool_list = tools
                    .iter()
                    .enumerate()
                    .map(|(idx, tool)| format!("{}. {}", idx + 1, tool))
                    .collect::<Vec<_>>()
                    .join("\n");

                Ok(format!(
                    "Active tools in this conversation ({}):\n{}",
                    tools.len(),
                    tool_list
                ))
            }
        }
        Err(e) => {
            error!("Failed to list tools for session '{}': {}", user.id, e);
            Err(format!("Failed to list tools: {}", e))
        }
    }
}
