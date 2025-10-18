use crate::shared::models::UserSession;
use crate::shared::state::AppState;
use diesel::prelude::*;
use log::{error, info, warn};
use rhai::{Dynamic, Engine};
use std::sync::Arc;
use uuid::Uuid;

pub fn add_tool_keyword(state: Arc<AppState>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();

    engine
        .register_custom_syntax(&["ADD_TOOL", "$expr$"], false, move |context, inputs| {
            let tool_path = context.eval_expression_tree(&inputs[0])?;
            let tool_path_str = tool_path.to_string().trim_matches('"').to_string();

            info!(
                "ADD_TOOL command executed: {} for session: {}",
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

            // Spawn async task to associate tool with session
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build();

                let send_err = if let Ok(rt) = rt {
                    let result = rt.block_on(async move {
                        associate_tool_with_session(
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
                    info!("ADD_TOOL completed: {}", message);
                    Ok(Dynamic::from(message))
                }
                Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    e.into(),
                    rhai::Position::NONE,
                ))),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "ADD_TOOL timed out".into(),
                        rhai::Position::NONE,
                    )))
                }
                Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("ADD_TOOL failed: {}", e).into(),
                    rhai::Position::NONE,
                ))),
            }
        })
        .unwrap();
}

/// Associate a compiled tool with the current session
/// The tool must already be compiled and present in the basic_tools table
async fn associate_tool_with_session(
    state: &AppState,
    user: &UserSession,
    tool_name: &str,
) -> Result<String, String> {
    use crate::shared::models::schema::{basic_tools, session_tool_associations};

    let mut conn = state.conn.lock().map_err(|e| {
        error!("Failed to acquire database lock: {}", e);
        format!("Database connection error: {}", e)
    })?;

    // First, verify the tool exists and is active for this bot
    let tool_exists: Result<bool, diesel::result::Error> = basic_tools::table
        .filter(basic_tools::bot_id.eq(user.bot_id.to_string()))
        .filter(basic_tools::tool_name.eq(tool_name))
        .filter(basic_tools::is_active.eq(1))
        .select(diesel::dsl::count(basic_tools::id))
        .first::<i64>(&mut *conn)
        .map(|count| count > 0);

    match tool_exists {
        Ok(true) => {
            info!(
                "Tool '{}' exists and is active for bot '{}'",
                tool_name, user.bot_id
            );
        }
        Ok(false) => {
            warn!(
                "Tool '{}' does not exist or is not active for bot '{}'",
                tool_name, user.bot_id
            );
            return Err(format!(
                "Tool '{}' is not available. Make sure the tool file is compiled and active.",
                tool_name
            ));
        }
        Err(e) => {
            error!("Failed to check tool existence: {}", e);
            return Err(format!("Database error while checking tool: {}", e));
        }
    }

    // Generate a unique ID for the association
    let association_id = Uuid::new_v4().to_string();
    let session_id_str = user.id.to_string();
    let added_at = chrono::Utc::now().to_rfc3339();

    // Insert the tool association (ignore if already exists due to UNIQUE constraint)
    let insert_result: Result<usize, diesel::result::Error> =
        diesel::insert_into(session_tool_associations::table)
            .values((
                session_tool_associations::id.eq(&association_id),
                session_tool_associations::session_id.eq(&session_id_str),
                session_tool_associations::tool_name.eq(tool_name),
                session_tool_associations::added_at.eq(&added_at),
            ))
            .on_conflict((
                session_tool_associations::session_id,
                session_tool_associations::tool_name,
            ))
            .do_nothing()
            .execute(&mut *conn);

    match insert_result {
        Ok(rows_affected) => {
            if rows_affected > 0 {
                info!(
                    "Tool '{}' newly associated with session '{}' (user: {}, bot: {})",
                    tool_name, user.id, user.user_id, user.bot_id
                );
                Ok(format!(
                    "Tool '{}' is now available in this conversation",
                    tool_name
                ))
            } else {
                info!(
                    "Tool '{}' was already associated with session '{}'",
                    tool_name, user.id
                );
                Ok(format!(
                    "Tool '{}' is already available in this conversation",
                    tool_name
                ))
            }
        }
        Err(e) => {
            error!(
                "Failed to associate tool '{}' with session '{}': {}",
                tool_name, user.id, e
            );
            Err(format!("Failed to add tool to session: {}", e))
        }
    }
}

/// Get all tools associated with a session
pub fn get_session_tools(
    conn: &mut PgConnection,
    session_id: &Uuid,
) -> Result<Vec<String>, diesel::result::Error> {
    use crate::shared::models::schema::session_tool_associations;

    let session_id_str = session_id.to_string();

    session_tool_associations::table
        .filter(session_tool_associations::session_id.eq(&session_id_str))
        .select(session_tool_associations::tool_name)
        .load::<String>(conn)
}

/// Remove a tool association from a session
pub fn remove_session_tool(
    conn: &mut PgConnection,
    session_id: &Uuid,
    tool_name: &str,
) -> Result<usize, diesel::result::Error> {
    use crate::shared::models::schema::session_tool_associations;

    let session_id_str = session_id.to_string();

    diesel::delete(
        session_tool_associations::table
            .filter(session_tool_associations::session_id.eq(&session_id_str))
            .filter(session_tool_associations::tool_name.eq(tool_name)),
    )
    .execute(conn)
}

/// Clear all tool associations for a session
pub fn clear_session_tools(
    conn: &mut PgConnection,
    session_id: &Uuid,
) -> Result<usize, diesel::result::Error> {
    use crate::shared::models::schema::session_tool_associations;

    let session_id_str = session_id.to_string();

    diesel::delete(
        session_tool_associations::table
            .filter(session_tool_associations::session_id.eq(&session_id_str)),
    )
    .execute(conn)
}
