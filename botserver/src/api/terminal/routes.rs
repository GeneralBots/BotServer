use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use futures_util::StreamExt;
use serde::Deserialize;
use uuid::Uuid;

use crate::security::auth_api::types::AuthenticatedUser;

use super::{sanitize_cwd, TerminalManager};
use botcore::shared::utils::DbPool;

#[derive(Deserialize)]
pub struct CreateTerminalRequest {
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub shell: Option<String>,
    /// Optional project VM container to exec into (e.g. `calculator-prod`).
    /// When set the shell runs inside that container via `incus exec`.
    #[serde(default)]
    pub container: Option<String>,
}

#[derive(Deserialize)]
pub struct KillTerminalRequest {
    pub id: String,
}

pub struct TerminalRouterState {
    pub manager: Arc<TerminalManager>,
    pub pool: DbPool,
}

pub fn configure_terminal_routes(pool: DbPool) -> Router {
    let manager = Arc::new(TerminalManager::new());
    Router::new()
        .route("/api/terminal/create", post(create_terminal))
        .route("/api/terminal/list", get(list_terminals))
        .route("/api/terminal/kill", post(kill_terminal))
        .route("/api/terminal/ws", get(terminal_ws))
        .with_state(Arc::new(TerminalRouterState { manager, pool }))
}

async fn create_terminal(
    State(state): State<Arc<TerminalRouterState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<CreateTerminalRequest>,
) -> impl IntoResponse {
    let container = req.container.clone().filter(|c| !c.trim().is_empty());
    // SECURITY: a container-less session spawns a PTY on the botserver host
    // itself — any caller could read the whole server filesystem ("terminal
    // exposes server contents" report). Web terminals must run inside a
    // project VM container; refuse anything else outright.
    let container = match container {
        Some(c) => c,
        None => {
            return (
                axum::http::StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "error": "Host shell denied: the terminal must run inside the project VM. Start the VM (Play) and retry."
                })),
            );
        }
    };
    // SECURITY: even with a container, only `vm_instances` containers of
    // projects the caller can access (viewer+) may be attached. Arbitrary
    // names (e.g. the server's own `tables`, `vault`, `drive`, `system`
    // containers) would otherwise grant an `incus exec` shell on production
    // infrastructure — the same "terminal exposes server contents" flaw in
    // another form.
    if let Err(e) = authorize_project_container(&state.pool, user.user_id, &container) {
        log::warn!("terminal create forbidden (user {}): {e}", user.user_id);
        return (
            axum::http::StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": e })),
        );
    }
    let shell = req
        .shell
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "/bin/bash".to_string());
    let cwd = sanitize_cwd(req.cwd.as_deref().unwrap_or(""));
    let result = state.manager.create_session_with_container(shell, cwd, Some(container));
    let (code, body) = match result {
        Ok(session) => (
            axum::http::StatusCode::OK,
            Json(serde_json::json!({
                "id": session.id,
                "shell": session.shell,
                "cwd": session.cwd,
                "container": session.container,
                "transport": if session.container.is_some() && cfg!(target_os = "windows") { "wsl-incus" } else if session.container.is_some() { "incus" } else { "local" },
                "created_at": session.created_at,
                "ws_url": format!("/api/terminal/ws?id={}", session.id),
            })),
        ),
        Err(e) => {
            log::error!("terminal create failed: {e}");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": botsecurity::error_sanitizer::log_and_sanitize_json(&e, "api::terminal::routes", None)})),
            )
        }
    };
    (code, body)
}

async fn list_terminals(
    State(state): State<Arc<TerminalRouterState>>,
) -> impl IntoResponse {
    state.manager.reap();
    Json(serde_json::json!({ "terminals": state.manager.list_sessions() }))
}

async fn kill_terminal(
    State(state): State<Arc<TerminalRouterState>>,
    Json(req): Json<KillTerminalRequest>,
) -> impl IntoResponse {
    match state.manager.kill_session(&req.id).await {
        Ok(()) => (
            axum::http::StatusCode::OK,
            Json(serde_json::json!({ "ok": true })),
        ),
        Err(e) => {
            log::warn!("terminal kill failed: {e}");
            (
                axum::http::StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e })),
            )
        }
    }
}

async fn terminal_ws(
    State(state): State<Arc<TerminalRouterState>>,
    Query(query): Query<std::collections::HashMap<String, String>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let id = query.get("id").cloned().unwrap_or_default();
    let session = match state.manager.get_session(&id) {
        Some(s) => s,
        None => {
            return axum::response::Response::builder()
                .status(axum::http::StatusCode::NOT_FOUND)
                .body("terminal session not found".into())
                .unwrap_or_else(|_| axum::response::Response::default());
        }
    };
    ws.on_upgrade(move |socket| terminal_ws_loop(socket, session))
}

async fn terminal_ws_loop(mut socket: WebSocket, session: Arc<super::TerminalSession>) {
    send_history(&session, &mut socket).await;
    let mut events_rx = session.events.subscribe();
    loop {
        tokio::select! {
            incoming = socket.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        if text == "ping" {
                            let _ = socket.send(Message::Text("pong".into())).await;
                            continue;
                        }
                        // xterm.js sends `resize <cols> <rows>`; forward it to
                        // the PTY instead of feeding it to the shell.
                        if let Some((cols, rows)) = parse_resize(&text) {
                            session.resize(cols, rows);
                            continue;
                        }
                        session.write(&text).ok();
                    }
                    Some(Ok(Message::Binary(bytes))) => {
                        session.write(&String::from_utf8_lossy(&bytes)).ok();
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(e)) => {
                        log::warn!("terminal ws error: {e}");
                        break;
                    }
                    _ => {}
                }
            }
            event = events_rx.recv() => {
                match event {
                    Ok(data) => {
                        let payload = serde_json::json!({ "type": "output", "data": data });
                        if socket.send(Message::Text(payload.to_string())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        log::warn!("terminal ws lagged {n} events");
                    }
                    Err(_) => break,
                }
            }
        }
    }
}

/// SECURITY gate for container attachment: the requested container must be a
/// `vm_instances` row for a project the caller can access (direct or group
/// membership with at least the `viewer` role).
fn authorize_project_container(
    pool: &DbPool,
    user_id: Uuid,
    container: &str,
) -> Result<(), String> {
    use diesel::prelude::*;
    if container.is_empty()
        || !container
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err("Invalid container name".to_string());
    }
    let mut conn = pool.get().map_err(|e| format!("db pool: {e}"))?;
    #[derive(diesel::QueryableByName)]
    struct UuidCell {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        value: Uuid,
    }
    let project_id: Option<UuidCell> = diesel::sql_query(
        "SELECT project_id AS value FROM vm_instances WHERE container_name = $1",
    )
    .bind::<diesel::sql_types::Text, _>(container)
    .get_result::<UuidCell>(&mut conn)
    .optional()
    .map_err(|e| format!("vm lookup: {e}"))?;
    let project_id = match project_id {
        Some(row) => row.value,
        None => {
            return Err(format!(
                "container '{container}' is not a project VM — only project VMs can be attached"
            ));
        }
    };
    if !project_role_at_least_viewer(&mut conn, user_id, project_id)? {
        return Err("you do not have access to this project's VM".to_string());
    }
    Ok(())
}

/// Resolves whether the caller holds at least the `viewer` role on the
/// project that owns the VM (direct membership or via an active group).
fn project_role_at_least_viewer(
    conn: &mut diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::PgConnection>>,
    user_id: Uuid,
    project_id: Uuid,
) -> Result<bool, String> {
    use diesel::prelude::*;
    if user_id == Uuid::nil() {
        return Ok(false);
    }
    #[derive(diesel::QueryableByName)]
    struct RoleCell {
        #[diesel(sql_type = diesel::sql_types::Text)]
        value: String,
    }
    let direct: Option<RoleCell> = diesel::sql_query(
        "SELECT role AS value FROM project_members WHERE project_id = $1 AND user_id = $2",
    )
    .bind::<diesel::sql_types::Uuid, _>(project_id)
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .get_result::<RoleCell>(conn)
    .optional()
    .map_err(|e| format!("project role lookup: {e}"))?;
    if let Some(role) = direct {
        return Ok(is_viewer_or_above(&role.value));
    }
    let groups: Vec<RoleCell> = diesel::sql_query(
        "SELECT g.name AS value FROM rbac_user_groups ug \
         JOIN rbac_groups g ON g.id = ug.group_id \
         WHERE ug.user_id = $1 AND g.is_active = true",
    )
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .load::<RoleCell>(conn)
    .map_err(|e| format!("user groups lookup: {e}"))?;
    for group in groups {
        let group_role: Option<RoleCell> = diesel::sql_query(
            "SELECT role AS value FROM project_members \
             WHERE project_id = $1 AND group_name = $2",
        )
        .bind::<diesel::sql_types::Uuid, _>(project_id)
        .bind::<diesel::sql_types::Text, _>(&group.value)
        .get_result::<RoleCell>(conn)
        .optional()
        .map_err(|e| format!("group role lookup: {e}"))?;
        if let Some(role) = group_role {
            if is_viewer_or_above(&role.value) {
                return Ok(true);
            }
        }
    }
    // Mirror the Vibe project RBAC semantics: an authenticated user with no
    // explicit membership on the project is treated as a viewer there (the
    // run/files endpoints resolve to the default `Viewer` role), so the
    // Terminal must grant the same access — otherwise a user who can already
    // Run a project's VM gets a 403 from its own companion terminal. The
    // container is still restricted to `vm_instances` rows, so infra
    // containers (tables, vault, system) remain unreachable.
    Ok(true)
}

fn is_viewer_or_above(role: &str) -> bool {
    matches!(
        role.to_ascii_lowercase().as_str(),
        "viewer" | "developer" | "admin" | "owner"
    )
}

/// Parses an xterm.js resize control message: `resize <cols> <rows>`.
fn parse_resize(text: &str) -> Option<(u16, u16)> {
    let rest = text.strip_prefix("resize ")?;
    let mut parts = rest.split_whitespace();
    let cols: u16 = parts.next()?.parse().ok()?;
    let rows: u16 = parts.next()?.parse().ok()?;
    Some((cols, rows))
}

async fn send_history(session: &Arc<super::TerminalSession>, socket: &mut WebSocket) {
    for line in session.history() {
        let payload = serde_json::json!({ "type": "output", "data": line.data });
        if socket
            .send(Message::Text(payload.to_string()))
            .await
            .is_err()
        {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_resize_accepts_valid_and_rejects_garbage() {
        assert_eq!(parse_resize("resize 120 40"), Some((120, 40)));
        assert_eq!(parse_resize("resize 0 0"), Some((0, 0)));
        assert_eq!(parse_resize("resize nope 40"), None);
        assert_eq!(parse_resize("echo hi"), None);
    }
}
