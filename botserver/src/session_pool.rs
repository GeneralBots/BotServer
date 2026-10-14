use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::PgConnection;

pub use botlib::db_pool::DbPool;
pub type DbConn = PooledConnection<ConnectionManager<PgConnection>>;

pub struct SessionPool {
    pub pool: DbPool,
}

impl SessionPool {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}
