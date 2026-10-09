use super::*;

/// Pool-based wrapper (Drive monitors, signup hook, backfill).
pub fn ensure_branch_default_project(
    pool: &botcore::shared::utils::DbPool,
    org_id: Uuid,
    branch_id: Uuid,
    name: &str,
) -> Result<Uuid, String> {
    let mut conn = pool.get().map_err(|e| format!("db pool: {e}"))?;
    ensure_branch_default_project_conn(&mut conn, org_id, branch_id, name)
}
