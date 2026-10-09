use super::*;

/// Resolves the real workspace branch for suite-mode callers that carry no
/// JWT (issue #734). Returns the branch flagged `is_default_for_branch`, or
/// `Uuid::nil()` (the fallback global scope) when no bot is flagged yet.
/// This replaces the previous hardcoded `Uuid::nil()` stub so suite-mode
/// handlers stop defaulting to the global shared scope.
pub(super) fn resolve_default_branch(conn: &mut diesel::PgConnection) -> uuid::Uuid {
    use diesel::prelude::*;
    if let Some(branch) = diesel::sql_query(
        "SELECT branch_id FROM bots WHERE is_default_for_branch = TRUE LIMIT 1",
    )
    .get_result::<DefaultBranchRow>(conn)
    .optional()
    .ok()
    .flatten()
    {
        return branch.branch_id;
    }
    uuid::Uuid::nil()
}
