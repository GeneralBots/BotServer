use super::*;

pub fn set_session_pool(pool: DbPool) {
    let _ = SESSION_POOL.set(pool);
}
