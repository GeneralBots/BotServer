use super::*;

pub async fn delete_connection(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> impl IntoResponse {
    if let Err(status) = require_authenticated(&user) {
        return (status, Json(serde_json::json!({ "success": false, "error": "Authentication required" })));
    }

    // Ownership check before removal (kill switch for non-owners is denied).
    let authorized = match state.session_manager.get_session(id).await {
        Some(s) => can_access_session(&user, &s),
        None => true, // Allow the delete to fall through to DB removal below.
    };
    if !authorized {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "success": false, "error": "not authorized to delete this connection" })),
        );
    }

    let removed_session = state.session_manager.remove_session(id).await;
    let user_for_task = user.user_id;

    // Also remove the saved row if it exists (scoped to the owner).
    let mut db_deleted = false;
    if let Some(pool) = state.pool.clone() {
        let pool_for_task = pool.clone();
        let result = tokio::task::spawn_blocking(move || -> Result<bool, String> {
            let mut conn = pool_for_task.get().map_err(|e| format!("db pool: {e}"))?;
            let affected = diesel::sql_query(
                "DELETE FROM desktop_connections WHERE id = $1 AND (user_id = $2 OR $3)",
            )
            .bind::<diesel::sql_types::Uuid, _>(id)
            .bind::<diesel::sql_types::Uuid, _>(user_for_task)
            .bind::<diesel::sql_types::Bool, _>(user.is_admin())
            .execute(&mut conn)
            .map_err(|e| format!("delete saved vdi connection: {e}"))?;
            Ok(affected > 0)
        })
        .await;
        if let Ok(Ok(d)) = result {
            db_deleted = d;
        }
    }

    // Audit: session end with actor + reason.
    if let Some(removed) = &removed_session {
        if let Some(pool) = state.pool.clone() {
            write_audit_event(
                &pool,
                &SessionAuditEvent {
                    connection_id: Uuid::new_v4(),
                    user_id: user_for_task,
                    session_id: removed.id,
                    host: removed.target_host.clone(),
                    port: removed.target_port,
                    protocol: removed.protocol.clone(),
                    connected_at: removed.created_at,
                    disconnected_at: Some(chrono::Utc::now()),
                    bytes_transferred: removed.bytes_sent + removed.bytes_received,
                    disconnect_reason: Some("deleted".to_string()),
                },
            );
        }
    }

    if removed_session.is_some() || db_deleted {
        info!("Connection deleted: {} by user {}", id, user_for_task);
        (StatusCode::OK, Json(serde_json::json!({ "success": true, "data": "deleted" })))
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "success": false, "error": format!("connection {id} not found") })),
        )
    }
}

// ---------------------------------------------------------------------------
// POST /connections/kill — kill switch for the caller's active sessions
// ---------------------------------------------------------------------------
pub async fn kill_own_sessions(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    if let Err(status) = require_authenticated(&user) {
        return (status, Json(serde_json::json!({ "success": false, "error": "Authentication required" })));
    }

    let removed = state.session_manager.kill_sessions_for_user(user.user_id).await;
    info!(
        "Kill switch: user {} terminated {} active session(s)",
        user.user_id,
        removed.len()
    );

    // Audit each terminated session.
    if let Some(pool) = state.pool.clone() {
        let now = chrono::Utc::now();
        for s in &removed {
            write_audit_event(
                &pool,
                &SessionAuditEvent {
                    connection_id: Uuid::new_v4(),
                    user_id: user.user_id,
                    session_id: s.id,
                    host: s.target_host.clone(),
                    port: s.target_port,
                    protocol: s.protocol.clone(),
                    connected_at: s.created_at,
                    disconnected_at: Some(now),
                    bytes_transferred: s.bytes_sent + s.bytes_received,
                    disconnect_reason: Some("kill-switch".to_string()),
                },
            );
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({ "success": true, "data": { "terminated": removed.len() } })),
    )
}

// ---------------------------------------------------------------------------
// GET /health — service health check
// ---------------------------------------------------------------------------
pub async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let total = state.session_manager.total_count().await;
    let body = serde_json::json!({
        "status": "ok",
        "active_connections": total,
    });
    (StatusCode::OK, Json(body))
}

// ---------------------------------------------------------------------------
// POST /health/tcp — TCP port probe
// ---------------------------------------------------------------------------
pub async fn health_check_tcp(
    Json(req): Json<crate::types::HealthCheckRequest>,
) -> impl IntoResponse {
    let (reachable, latency_ms, err) = tcp_health_check(&req.host, req.port).await;
    let resp = HealthCheckResponse {
        reachable,
        latency_ms,
        error: err,
    };
    (StatusCode::OK, Json(serde_json::json!({ "success": true, "data": resp })))
}

// ---------------------------------------------------------------------------
// POST /health/rdp — RDP-specific probe (confirms an RDP server is listening)
// ---------------------------------------------------------------------------
pub async fn health_check_rdp(
    Json(req): Json<crate::types::HealthCheckRequest>,
) -> impl IntoResponse {
    let (reachable, latency_ms, err) = rdp_health_check(&req.host, req.port).await;
    let resp = HealthCheckResponse {
        reachable,
        latency_ms,
        error: err,
    };
    (StatusCode::OK, Json(serde_json::json!({ "success": true, "data": resp })))
}

// ---------------------------------------------------------------------------
// GET /ws/{session_id} — WebSocket proxy endpoint
// ---------------------------------------------------------------------------
pub async fn ws_proxy_handler(
    ws: WebSocketUpgrade,
    axum::extract::Path(session_id): axum::extract::Path<Uuid>,
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
) -> impl IntoResponse {
    // Browser WebSocket handshakes cannot carry an Authorization header, so
    // the random 128-bit `session_id` acts as a capability token (same model
    // as the terminal WS). Sessions are only ever created by authenticated
    // users (`create_connection`/`handle_connect` reject anonymous callers),
    // so a valid session id already implies an authenticated owner.
    let session = match state.session_manager.get_session(session_id).await {
        Some(s) => s,
        None => {
            warn!("WS proxy requested for unknown session: {}", session_id);
            return (StatusCode::NOT_FOUND, "session not found").into_response();
        }
    };

    // WebSocket handshakes from the browser cannot send an Authorization
    // header, so the session id is the capability. Sessions are created only
    // by authenticated users and the owner check happens at creation time
    // (`create_connection`/`handle_connect`); the proxy itself validates the
    // session exists and is not expired.

    let client_ip = extract_client_ip(&addr);
    debug!(
        "WS upgrade: session={} client={}",
        session_id, mask_ip(&client_ip),
    );

    ws.on_upgrade(move |socket| async move {
        handle_ws_connection(socket, session, state.session_manager).await;
    })
    .into_response()
}

// ---------------------------------------------------------------------------
// Axum router builder
// ---------------------------------------------------------------------------
