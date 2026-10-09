use super::*;

pub(super) fn get_conn(
    state: &Arc<DirectoryApiState>,
) -> Option<diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::PgConnection>>> {
    state.conn.get().ok()
}
