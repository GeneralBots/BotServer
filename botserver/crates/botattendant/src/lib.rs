pub mod handlers;
pub mod scope;
pub mod models;
pub mod routes;
pub mod schema;
pub mod ui;

use std::sync::Arc;
use uuid::Uuid;

pub use botlib::db_pool::DbPool;

pub type GetDefaultBotFn = fn(&mut diesel::PgConnection) -> Uuid;

pub struct AttendantConfig {
    pub pool: Arc<DbPool>,
    pub get_default_bot: GetDefaultBotFn,
}

pub use routes::configure_attendant_routes;
pub use models::*;
pub use ui::configure_attendant_ui_routes;
