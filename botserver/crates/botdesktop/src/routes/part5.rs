use super::*;

pub fn router(state: AppState) -> axum::Router {
    axum::Router::new()
        .route(&format!("{PREFIX}/connections"), axum::routing::post(create_connection))
        .route(&format!("{PREFIX}/connections"), axum::routing::get(list_connections))
        .route(&format!("{PREFIX}/connections/kill"), axum::routing::post(kill_own_sessions))
        .route(&format!("{PREFIX}/connections/{{id}}"), axum::routing::get(get_connection))
        .route(&format!("{PREFIX}/connections/{{id}}"), axum::routing::delete(delete_connection))
        .route(&format!("{PREFIX}/connect"), axum::routing::post(handle_connect))
        .route(&format!("{PREFIX}/health"), axum::routing::get(health_check))
        .route(&format!("{PREFIX}/health/tcp"), axum::routing::post(health_check_tcp))
        .route(&format!("{PREFIX}/health/rdp"), axum::routing::post(health_check_rdp))
        .route(&format!("{PREFIX}/ws/proxy/{{session_id}}"), axum::routing::get(ws_proxy_handler))
        .with_state(state)
}
impl Default for AppState {
    fn default() -> Self {
        Self::new(None)
    }
}
impl AppState {
    pub fn new(pool: Option<DbPool>) -> Self {
        let config = ConnectionConfig::default();
        Self {
            session_manager: SessionManager::new(config.clone()),
            config,
            pool,
        }
    }
}
/// Read the default VDI connection from Vault (`secret/gbo/vdi`).
/// Never hardcodes infrastructure addresses in the repository; the host is
/// provisioned at deploy time as Vault fields `default-host`/`default-port`/
/// `default-name`.
pub(super) fn vdi_default_connection() -> Option<(String, u16, String)> {
    let manager = botcoresecrets::SecretsManager::get_clone().ok()?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        let result = match rt {
            Ok(rt) => {
                rt.block_on(manager.get_secret(botcoresecrets::SecretPaths::VDI))
            }
            Err(e) => {
                warn!("vdi: failed to create runtime: {e}");
                return;
            }
        };
        let _ = tx.send(result);
    });
    let secrets = rx.recv().ok()?.ok()?;

    let host = secrets.get("default-host").filter(|h| !h.is_empty())?.clone();
    let port = secrets
        .get("default-port")
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(5900);
    let name = secrets
        .get("default-name")
        .filter(|n| !n.is_empty())
        .cloned()
        .unwrap_or_else(|| "Default Desktop".to_string());
    Some((host, port, name))
}
/// Seed the default VDI connection into `desktop_connections` from Vault.
/// Idempotent (`ON CONFLICT DO NOTHING`); no-op when the Vault path is absent.
pub fn seed_default_connection(pool: &DbPool) {
    let Some((host, port, name)) = vdi_default_connection() else {
        return;
    };

    let result = (|| -> Result<(), String> {
        let mut conn = pool.get().map_err(|e| format!("db pool: {e}"))?;
        diesel::sql_query(
            "INSERT INTO desktop_connections \
             (id, user_id, name, host, port, protocol, auth_type, auto_connect, created_at, updated_at, last_used_at) \
             VALUES ($1, $2, $3, $4, $5, 'vnc', 'password', true, $6, $6, $6) \
             ON CONFLICT DO NOTHING",
        )
        .bind::<diesel::sql_types::Uuid, _>(Uuid::new_v4())
        .bind::<diesel::sql_types::Uuid, _>(Uuid::nil())
        .bind::<diesel::sql_types::Text, _>(&name)
        .bind::<diesel::sql_types::Text, _>(&host)
        .bind::<diesel::sql_types::Int4, _>(port as i32)
        .bind::<diesel::sql_types::Timestamptz, _>(chrono::Utc::now())
        .execute(&mut conn)
        .map_err(|e| format!("seed default vdi connection: {e}"))?;
        Ok(())
    })();
    if let Err(e) = result {
        tracing::warn!("Failed to seed default VDI connection for {host}: {e}");
    } else {
        info!("Seeded default VDI connection from Vault secret/gbo/vdi");
    }
}
pub fn configure_routes(pool: Option<DbPool>) -> axum::Router<()> {
    if let Some(p) = &pool {
        seed_default_connection(p);
    }
    router(AppState::new(pool))
}
