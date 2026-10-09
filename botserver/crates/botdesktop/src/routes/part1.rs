use super::*;

/// Returns true when the protocol is a supported remote-desktop protocol.
pub(super) fn is_supported_protocol(protocol: &str) -> bool {
    SUPPORTED_PROTOCOLS.contains(&protocol)
}
/// Stores RDP target credentials in Vault and returns the vault path, or
/// `None` when no secret needs vaulting (no password supplied).
///
/// The path is scoped per user so one tenant can never read another's
/// credentials: `gbo/orgs/{org_id}/vdi/{connection_id}`.
pub(super) fn vault_vdi_credentials(
    user_id: Uuid,
    org_id: Option<Uuid>,
    connection_id: Uuid,
    password: Option<&str>,
    domain: Option<&str>,
) -> Option<String> {
    let password = password.unwrap_or("");
    let domain = domain.unwrap_or("");
    if password.is_empty() && domain.is_empty() {
        return None;
    }

    let org = org_id.map(|o| o.to_string()).unwrap_or_else(|| "default".to_string());
    let path = format!("gbo/orgs/{org}/vdi/{connection_id}");

    let manager = match botcoresecrets::SecretsManager::get_clone() {
        Ok(m) => m,
        Err(e) => {
            warn!("vdi: Vault unavailable, RDP credentials for {path} not vaulted: {e}");
            return None;
        }
    };
    let mut secrets = std::collections::HashMap::new();
    if !password.is_empty() {
        secrets.insert("password".to_string(), password.to_string());
    }
    if !domain.is_empty() {
        secrets.insert("domain".to_string(), domain.to_string());
    }

    let (tx, rx) = std::sync::mpsc::channel();
    let manager_clone = manager.clone();
    let path_clone = path.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build();
        let result = match rt {
            Ok(rt) => rt.block_on(manager_clone.put_secret(&path_clone, secrets)),
            Err(e) => {
                warn!("vdi: failed to build runtime for credential storage: {e}");
                return;
            }
        };
        let _ = tx.send(result);
    });

    match rx.recv() {
        Ok(Ok(())) => {
            info!("vdi: RDP credentials for {connection_id} vaulted at {path} (user {user_id})");
            Some(path)
        }
        Ok(Err(e)) => {
            warn!("vdi: failed to store RDP credentials at {path}: {e}");
            None
        }
        Err(_) => {
            warn!("vdi: credential storage channel closed for {path}");
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Authentication & authorization helpers
// ---------------------------------------------------------------------------
/// Rejects unauthenticated callers (anonymous users or the nil UUID).
/// Returns `Ok(user)` for authenticated callers, `Err(status)` otherwise.
pub(super) fn require_authenticated(user: &AuthenticatedUser) -> Result<(), StatusCode> {
    if !user.is_authenticated() {
        warn!("Desktop proxy: rejected unauthenticated request (user={})", user.user_id);
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(())
}
/// Applies RBAC: a user may only reach sessions they own; admins may reach
/// any session.
pub(super) fn can_access_session(user: &AuthenticatedUser, session: &DesktopSession) -> bool {
    user.is_admin() || session.is_owned_by(user.user_id)
}
/// Persists a session audit event into `desktop_connection_log`.
pub(super) fn write_audit_event(pool: &DbPool, event: &SessionAuditEvent) {
    let pool = pool.clone();
    let event = event.clone();
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let mut conn = pool.get().map_err(|e| format!("db pool: {e}"))?;
        diesel::sql_query(
            "INSERT INTO desktop_connection_log \
             (connection_id, user_id, session_id, host, port, protocol, connected_at, \
              disconnected_at, bytes_transferred, disconnect_reason) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind::<diesel::sql_types::Uuid, _>(event.connection_id)
        .bind::<diesel::sql_types::Uuid, _>(event.user_id)
        .bind::<diesel::sql_types::Uuid, _>(event.session_id)
        .bind::<diesel::sql_types::Text, _>(&event.host)
        .bind::<diesel::sql_types::Int4, _>(event.port as i32)
        .bind::<diesel::sql_types::Text, _>(&event.protocol)
        .bind::<diesel::sql_types::Timestamptz, _>(event.connected_at)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>, _>(event.disconnected_at)
        .bind::<diesel::sql_types::BigInt, _>(event.bytes_transferred)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(event.disconnect_reason)
        .execute(&mut conn)
        .map_err(|e| format!("write desktop audit event: {e}"))?;
        Ok(())
    });
}
