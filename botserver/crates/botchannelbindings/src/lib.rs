mod api;
pub mod models;
pub mod schema;


pub use botlib::db_pool::DbPool;

#[derive(Clone)]
pub struct ChannelBindingsService {
    pub pool: DbPool,
}

impl ChannelBindingsService {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

pub use api::configure_routes;
