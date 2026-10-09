use super::*;

pub fn configure_sources_ui_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/suite/sources", get(handle_sources_list_page))
        .route("/suite/sources/mcp/add", get(mcp_add::handle_mcp_add_page))
        .route(
            "/suite/sources/mcp/catalog",
            get(mcp_catalog::handle_mcp_catalog_page),
        )
}
