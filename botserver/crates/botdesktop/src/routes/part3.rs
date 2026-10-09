use super::*;

pub async fn handle_connect(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<ConnectRequest>,
) -> impl IntoResponse {
    let client_ip = extract_client_ip(&addr);
    let user_id = user.user_id;

    if let Err(status) = require_authenticated(&user) {
        return (status, Json(serde_json::json!({ "success": false, "error": "Authentication required" })));
    }

    if req.host.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "success": false, "error": "host is required" })),
        );
    }
    if req.port == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "success": false, "error": "port must be between 1 and 65535" })),
        );
    }
    if !ConnectionConfig::is_target_port_allowed(req.port) {
        warn!(
            "Desktop proxy: user {} attempted to proxy to disallowed port {} (allow-list: VNC 5900-5999, RDP 3389)",
            user_id, req.port
        );
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "success": false, "error": "port is not on the allowed list (VNC 5900-5999, RDP 3389)" })),
        );
    }

    let protocol = req.protocol.clone().unwrap_or_else(|| "vnc".to_string());
    if !is_supported_protocol(&protocol) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "success": false, "error": "unsupported protocol (expected vnc or rdp)" })),
        );
    }

    let session = DesktopSession::with_protocol(
        user_id,
        user.organization_id,
        None,
        req.host.clone(),
        req.port,
        &protocol,
        client_ip,
    );

    // RDP credentials go to Vault, never to the DB or UI.
    let vault_path = if protocol == "rdp" {
        vault_vdi_credentials(
            user_id,
            user.organization_id,
            session.id,
            req.password.as_deref(),
            req.domain.as_deref(),
        )
    } else {
        None
    };

    let auth_type = req.auth_type.clone().unwrap_or_else(|| "password".to_string());
    let conn_name = req.name.clone();
    let target_host = req.host.clone();
    let target_port = req.port;
    let proto = protocol.clone();
    let domain = req.domain.clone();

    // Persist the saved connection so it survives restarts (#vdi persistence).
    // RDP credentials live in Vault; the row keeps only the vault path.
    if let Some(pool) = state.pool.clone() {
        let pool_for_task = pool.clone();
        let host_for_task = target_host.clone();
        let name_for_task = conn_name.clone();
        let auth_for_task = auth_type.clone();
        let proto_for_task = proto.clone();
        let vault_path_for_task = vault_path.clone();
        let domain_for_task = domain.clone();
        let now = chrono::Utc::now();
        let log_name = name_for_task.clone();
        let log_host = host_for_task.clone();
        let result = tokio::task::spawn_blocking(move || -> Result<(), String> {
            let mut conn = pool_for_task.get().map_err(|e| format!("db pool: {e}"))?;
            diesel::sql_query(
                "INSERT INTO desktop_connections \
                 (id, user_id, name, host, port, protocol, auth_type, auto_connect, secrets_vault_path, rdp_domain, created_at, updated_at, last_used_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, true, $8, $9, $10, $10, $10) \
                 ON CONFLICT DO NOTHING",
            )
            .bind::<diesel::sql_types::Uuid, _>(Uuid::new_v4())
            .bind::<diesel::sql_types::Uuid, _>(user_id)
            .bind::<diesel::sql_types::Text, _>(&name_for_task)
            .bind::<diesel::sql_types::Text, _>(&host_for_task)
            .bind::<diesel::sql_types::Int4, _>(target_port as i32)
            .bind::<diesel::sql_types::Text, _>(&proto_for_task)
            .bind::<diesel::sql_types::Text, _>(&auth_for_task)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&vault_path_for_task)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&domain_for_task)
            .bind::<diesel::sql_types::Timestamptz, _>(now)
            .execute(&mut conn)
            .map_err(|e| format!("persist vdi connection: {e}"))?;
            Ok(())
        })
        .await;
        if let Err(e) = result {
            tracing::warn!("Failed to persist VDI connection {} -> {}: {}", log_name, log_host, e);
        }
    }

    // Audit: session start for the VDI connect flow (protocol-aware).
    if let Some(pool) = state.pool.clone() {
        let audit_session = session.clone();
        write_audit_event(
            &pool,
            &SessionAuditEvent {
                connection_id: Uuid::new_v4(),
                user_id,
                session_id: audit_session.id,
                host: audit_session.target_host.clone(),
                port: audit_session.target_port,
                protocol: audit_session.protocol.clone(),
                connected_at: audit_session.created_at,
                disconnected_at: None,
                bytes_transferred: 0,
                disconnect_reason: None,
            },
        );
    }

    match state.session_manager.register_session(session).await {
        Ok(session) => {
            info!("VDI connect created: {} -> {}:{} ({})", req.name, req.host, req.port, session.protocol);
            (
                StatusCode::CREATED,
                Json(serde_json::json!({
                    "success": true,
                    "data": session.to_summary(),
                    "name": req.name,
                    "protocol": session.protocol,
                })),
            )
        }
        Err(SessionError::RateLimitExceeded { max, .. }) => (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({
                "success": false,
                "error": format!("maximum {max} concurrent connections per user"),
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "error": e.to_string(),
            })),
        ),
    }
}

// ---------------------------------------------------------------------------
// GET /connections — list all active connections
// ---------------------------------------------------------------------------
pub async fn list_connections(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    if let Err(status) = require_authenticated(&user) {
        return (status, Json(serde_json::json!({ "success": false, "error": "Authentication required" })));
    }

    // Session listing is scoped to the owning user (admins see all).
    let sessions = state.session_manager.get_all_sessions().await;
    let mut summaries: Vec<serde_json::Value> = sessions
        .iter()
        .filter(|s| user.is_admin() || s.is_owned_by(user.user_id))
        .map(|s| serde_json::json!(s.to_summary()))
        .collect();

    // Merge saved (persisted) connections so the grid shows them after restart.
    if let Some(pool) = state.pool.clone() {
        let pool_for_task = pool.clone();
        let user_for_task = user.user_id;
        let is_admin = user.is_admin();
        let result = tokio::task::spawn_blocking(move || -> Result<Vec<serde_json::Value>, String> {
            let mut conn = pool_for_task.get().map_err(|e| format!("db pool: {e}"))?;
            #[derive(diesel::QueryableByName)]
            struct SavedRow {
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                id: Uuid,
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                user_id: Uuid,
                #[diesel(sql_type = diesel::sql_types::Text)]
                name: String,
                #[diesel(sql_type = diesel::sql_types::Text)]
                host: String,
                #[diesel(sql_type = diesel::sql_types::Int4)]
                port: i32,
                #[diesel(sql_type = diesel::sql_types::Text)]
                protocol: String,
            }
            let rows: Vec<SavedRow> = if is_admin {
                diesel::sql_query(
                    "SELECT id, user_id, name, host, port, protocol FROM desktop_connections ORDER BY name ASC",
                )
                .load::<SavedRow>(&mut conn)
            } else {
                diesel::sql_query(
                    "SELECT id, user_id, name, host, port, protocol FROM desktop_connections WHERE user_id = $1 ORDER BY name ASC",
                )
                .bind::<diesel::sql_types::Uuid, _>(user_for_task)
                .load::<SavedRow>(&mut conn)
            }
            .map_err(|e| format!("load saved vdi connections: {e}"))?;
            Ok(rows
                .into_iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.id,
                        "user_id": r.user_id,
                        "target_host": r.host,
                        "target_port": r.port,
                        "status": "saved",
                        "name": r.name,
                        "protocol": r.protocol,
                        "saved": true,
                    })
                })
                .collect())
        })
        .await;
        if let Ok(Ok(mut saved)) = result {
            summaries.append(&mut saved);
        }
    }

    (StatusCode::OK, Json(serde_json::json!({ "success": true, "data": summaries })))
}

// ---------------------------------------------------------------------------
// GET /connections/{id} — single connection detail
// ---------------------------------------------------------------------------
pub async fn get_connection(
    State(state): State<AppState>,
    Extension(user): Extension<AuthenticatedUser>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> impl IntoResponse {
    if let Err(status) = require_authenticated(&user) {
        return (status, Json(serde_json::json!({ "success": false, "error": "Authentication required" })));
    }

    match state.session_manager.get_session(id).await {
        Some(s) if can_access_session(&user, &s) => {
            (StatusCode::OK, Json(serde_json::json!({ "success": true, "data": s.to_summary() })))
        }
        Some(_) => (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "success": false, "error": "not authorized to view this connection" })),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "success": false, "error": format!("connection {id} not found") })),
        ),
    }
}

// ---------------------------------------------------------------------------
// DELETE /connections/{id} — terminate a connection
// ---------------------------------------------------------------------------
