use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;

pub mod error;
pub mod handlers;
pub mod schema;
pub mod storage;
pub mod types;
pub mod ui;

pub use error::DashboardsError;
pub use handlers::*;
pub use storage::*;
pub use types::*;

pub use botlib::db_pool::DbPool;

pub type GetDefaultBotFn = fn(&mut diesel::PgConnection) -> uuid::Uuid;

#[derive(Clone)]
pub struct DashboardsState {
    pub pool: DbPool,
    pub get_default_bot: GetDefaultBotFn,
}

pub fn configure_dashboards_routes(state: Arc<DashboardsState>) -> Router<Arc<DashboardsState>> {
    Router::new()
        .route("/api/dashboards", get(handle_list_dashboards).post(handle_create_dashboard))
        .route("/api/dashboards/templates", get(handle_get_templates))
        .route("/api/dashboards/:id", get(handle_get_dashboard).put(handle_update_dashboard).delete(handle_delete_dashboard))
        .route("/api/dashboards/:id/widgets", post(handle_add_widget))
        .route("/api/dashboards/:id/widgets/:widget_id", put(handle_update_widget).delete(handle_delete_widget))
        .route("/api/dashboards/:id/widgets/:widget_id/data", get(handle_get_widget_data))
        .route("/api/dashboards/sources", get(handle_list_data_sources).post(handle_create_data_source))
        .route("/api/dashboards/sources/:id/test", post(handle_test_data_source))
        .route("/api/dashboards/sources/:id", delete(handle_delete_data_source))
        .route("/api/dashboards/data-sources", get(handle_list_data_sources).post(handle_create_data_source))
        .route("/api/dashboards/data-sources/test", post(handle_test_data_source_no_id))
        .route("/api/dashboards/query", post(handle_conversational_query))
        .with_state(state)
}
