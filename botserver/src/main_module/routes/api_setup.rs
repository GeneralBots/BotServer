use axum::{Router, routing::{get, post}};
use std::sync::Arc;
use botcore::shared::state::AppState;
use botcore::urls::ApiUrls;

pub fn setup_api_routes() -> Router<Arc<AppState>> {
    let mut api_router = Router::new()
        .route("/health", get(crate::health_check_simple))
        .route(ApiUrls::HEALTH, get(crate::health_check))
        .route("/api/config/reload", post(botcore::config_reload::reload_config))
        .route("/api/product", get(super::product_handlers::get_product_config))
        .route("/api/manifest", get(super::product_handlers::get_workspace_manifest))
        .route("/api/client-errors", post(crate::receive_client_errors))
        .route("/api/bot/config", get(crate::core::bot::get_bot_config))
        .route("/api/bot/public", get(crate::core::bot::get_public_bot_config))
        .route("/api/bots/:bot_name/access", get(crate::core::bot::check_access_handler))
        .route(ApiUrls::SESSIONS, post(crate::core::session::create_session))
        .route(ApiUrls::SESSION_START, post(crate::core::session::start_session))
        .route(ApiUrls::WS, get(crate::core::bot::websocket_handler))
        .route("/ws/:bot_name", get(crate::core::bot::websocket_handler_with_bot))
        // AutoTask progress stream — botui proxies /ws/task-progress[/:task_id]
        // here (issue #1266: the endpoint was missing so every client dial
        // failed and progress events never reached the UI).
        .route(
            "/ws/task-progress",
            get(super::task_progress_ws::task_progress_ws),
        )
        .route(
            "/ws/task-progress/:task_id",
            get(super::task_progress_ws::task_progress_ws_with_id),
        );

    #[cfg(feature = "drive")]
    {
        use axum::routing::{get as axum_get, post as axum_post};
        use crate::security::rbac_middleware::require_admin_middleware;

        // Admin-only drive routes (bot management)
        let admin_drive_routes = axum::Router::new()
            .route("/api/files/bots", axum_post(crate::drive::drive_handlers::create_bot))
            .route("/api/files/bots/delete", axum_post(crate::drive::drive_handlers::delete_bot))
            .route("/api/files/buckets", axum_get(crate::drive::drive_handlers::list_buckets))
            .layer(axum::middleware::from_fn(require_admin_middleware));
        api_router = api_router.merge(admin_drive_routes);

        // Regular authenticated drive routes
        api_router = api_router
            .route("/api/files/list", axum_get(crate::drive::drive_handlers::list_files))
            .route("/api/files/quota", axum_get(crate::drive::drive_handlers::quota))
            .route("/api/files/recent", axum_get(crate::drive::drive_handlers::recent_files))
            .route("/api/files/search", axum_get(crate::drive::drive_handlers::search_files))
            .route("/api/files/write", axum_post(crate::drive::drive_handlers::upload_file_to_drive))
            .route("/api/files/read", axum_post(crate::drive::drive_handlers::read_file))
            .route("/api/files/download", axum_post(crate::drive::drive_handlers::download_file))
            .route("/api/files/download-binary", axum_post(crate::drive::drive_handlers::download_file_binary))
            .route("/api/files/download-inline", axum_post(crate::drive::drive_handlers::download_file_inline))
            .route("/api/files/delete", axum_post(crate::drive::drive_handlers::delete_file))
            .route("/api/files/createFolder", axum_post(crate::drive::drive_handlers::create_folder))
            .route("/api/files/copy", axum_post(crate::drive::drive_handlers::copy_file))
            .route("/api/files/move", axum_post(crate::drive::drive_handlers::move_file))
            .route("/api/files/open", axum_post(crate::drive::drive_handlers::open_file))
            .route("/api/files/ai/chat", axum_post(crate::drive::drive_handlers::ai_chat_handler))
            .route("/api/files/favorite", axum_get(crate::drive::drive_handlers::list_favorites))
            .route("/api/files/favorite/toggle", axum_post(crate::drive::drive_handlers::toggle_star))
            .route("/api/files/shared", axum_get(crate::drive::drive_handlers::list_shared))
            .route("/api/files/share", axum_post(crate::drive::drive_handlers::share_folder))
            .route("/api/files/trash", axum_get(crate::drive::drive_handlers::list_trash))
            .route("/api/files/trash/move", axum_post(crate::drive::drive_handlers::trash_file))
            .route("/api/files/trash/restore", axum_post(crate::drive::drive_handlers::restore_trash))
            .route("/api/files/trash/empty", axum_post(crate::drive::drive_handlers::empty_trash))
            .route("/api/files/upload-binary", axum_post(crate::drive::drive_handlers::upload_file_binary))
            // Public share links (token-based). The download endpoint is
            // registered here but served anonymously via the
            // /api/files/public allowlist in security_setup.rs.
            .route("/api/files/share-link", axum_post(crate::drive::drive_handlers::create_public_link))
            .route("/api/files/share-links", axum_get(crate::drive::drive_handlers::list_public_links))
            .route("/api/files/revoke-link", axum_post(crate::drive::drive_handlers::revoke_public_link))
            .route("/api/files/public/:token", axum_get(crate::drive::drive_handlers::public_link_download));
    }

    api_router = api_router
        .route(ApiUrls::AUTH, get(super::anonymous_auth::anonymous_auth_handler));

    // External drives (OneDrive / Google Drive) — the Drive app's External tab.
    // Merged here (not in the feature-gated drive block) because the backing
    // tables are created at boot regardless of the `drive` feature.
    api_router = api_router.merge(botdrive::external::routes::configure());

    // Public catalog API — no auth required (before auth middleware)
    api_router = api_router.merge(super::catalog::configure_catalog_routes());

    api_router
}

pub fn add_base_api_routes(mut api_router: Router<Arc<AppState>>) -> Router<Arc<AppState>> {
    use super::org_handlers::*;
    use super::cloud_sso_handler::{handle_cloud_sso, handle_suite_sso, handle_unified_login};

    // Cross-app collaboration layer (threaded comments, reactions, presence).
    api_router = api_router.merge(super::collab_routes::configure_collab_routes());

    api_router
        .nest("/api/auth", crate::directory::auth_routes::configure())
        .route("/api/auth/suite-sso", get(handle_suite_sso))
        .route("/api/auth/cloud-sso", post(handle_cloud_sso))
        .route("/api/auth/unified-login", post(handle_unified_login))
        .route("/api/organizations/current", get(handle_get_organization).put(handle_update_organization).post(handle_update_organization).delete(handle_delete_organization))
        .route("/api/organizations", post(handle_create_organization))
        .route("/api/organizations/current/settings", get(handle_get_org_settings))
        .route("/api/organizations/current/stats", get(handle_get_org_stats))
        .route("/api/organizations/current/contact", post(handle_update_organization_contact))
        .route("/api/organizations/current/branding", post(handle_update_organization_branding))
        .route("/api/organizations/current/audit", get(handle_get_org_audit))
        .route("/api/organizations/current/export", get(handle_export_org_data))
        .route("/api/admin/migrate/office365", post(handle_office365_migration))
}
