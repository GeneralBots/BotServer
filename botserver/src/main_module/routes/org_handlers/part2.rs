use super::*;

pub async fn handle_office365_migration(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<Office365MigrationRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    info!("Office 365 migration requested for tenant: {}", req.tenant_id);

    let mode = match req.sync_mode.as_deref() {
        Some("delta") => crate::directory::scim::sync::SyncMode::Delta,
        _ => crate::directory::scim::sync::SyncMode::Full,
    };

    let config = crate::directory::scim::sync::AzureAdConfig {
        tenant_id: req.tenant_id,
        client_id: req.client_id,
        client_secret: req.client_secret,
        sync_mode: mode,
    };

    let syncer = crate::directory::scim::sync::AzureAdSyncer::new(config);

    let auth_service = _state.auth_service.as_ref().ok_or_else(|| {
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "error": "No auth service available"
        })))
    })?.lock().await;

    match syncer.sync(&*auth_service).await {
        Ok(result) => {
            info!("Office 365 migration complete: {:?}", result);
            Ok(Json(serde_json::json!({
                "success": true,
                "groups_created": result.groups_created,
                "groups_updated": result.groups_updated,
                "users_mapped": result.users_mapped,
                "users_created": result.users_created,
                "users_updated": result.users_updated,
                "errors": result.errors,
                "duration_ms": result.duration_ms
            })))
        }
        Err(e) => {
            log::error!("Office 365 migration failed: {}", e);
            Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Migration failed: {}", e)
                }))
            ))
        }
    }
}
