use super::*;

pub async fn create_connection(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<ConnectionCreateRequest>,
) -> impl IntoResponse {
    let client_ip = extract_client_ip(&addr);
    let user_id = user.user_id;

    if let Err(status) = require_authenticated(&user) {
        return (status, Json(serde_json::json!({ "success": false, "error": "Authentication required" })));
    }

    debug!(
        "create_connection: user={} target={}:{} client={}",
        user_id, req.target_host, req.target_port, mask_ip(&client_ip),
    );

    if req.target_host.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "success": false, "error": "target_host is required" })),
        );
    }
    if req.target_port == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "success": false, "error": "target_port must be between 1 and 65535" })),
        );
    }
    if !ConnectionConfig::is_target_port_allowed(req.target_port) {
        warn!(
            "Desktop proxy: user {} attempted to proxy to disallowed port {} (allow-list: VNC 5900-5999, RDP 3389)",
            user_id, req.target_port
        );
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "success": false, "error": "target port is not on the allowed list (VNC 5900-5999, RDP 3389)" })),
        );
    }
    if !is_supported_protocol(&req.protocol) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "success": false, "error": "unsupported protocol (expected vnc or rdp)" })),
        );
    }

    // Tenant scoping: sessions are attributed to the authenticated user and
    // their organization/branch when available.
    let session = DesktopSession::with_protocol(
        user_id,
        user.organization_id,
        None,
        req.target_host.clone(),
        req.target_port,
        &req.protocol,
        client_ip,
    );

    // RDP credentials are vaulted server-side; the session keeps only the
    // vault path (never the password itself).
    if req.protocol == "rdp" {
        let _ = vault_vdi_credentials(
            user_id,
            user.organization_id,
            session.id,
            req.password.as_deref(),
            req.domain.as_deref(),
        );
    }

    match state.session_manager.register_session(session).await {
        Ok(session) => {
            info!(
                "Connection created: {} (user={}, target={}:{}, protocol={})",
                session.id, user_id, req.target_host, req.target_port, session.protocol,
            );
            // Audit: session start with actor + target (masked in logs).
            if let Some(pool) = state.pool.clone() {
                write_audit_event(
                    &pool,
                    &SessionAuditEvent {
                        connection_id: Uuid::new_v4(),
                        user_id,
                        session_id: session.id,
                        host: session.target_host.clone(),
                        port: session.target_port,
                        protocol: session.protocol.clone(),
                        connected_at: session.created_at,
                        disconnected_at: None,
                        bytes_transferred: 0,
                        disconnect_reason: None,
                    },
                );
            }
            (
                StatusCode::CREATED,
                Json(serde_json::json!({
                    "success": true,
                    "data": session.to_summary(),
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
                "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botdesktop::routes::part2", None),
            })),
        ),
    }
}

// ---------------------------------------------------------------------------
// POST /connect — create a persisted-style connection (maps to a proxy session)
// ---------------------------------------------------------------------------
