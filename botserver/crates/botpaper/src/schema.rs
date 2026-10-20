// @generated automatically by script from migration SQL for issue #707.
// Diesel table definitions for branch-scope cleanup.

// Canonical definition lives in `botschema` (#1497).
pub use botschema::user_sessions;


diesel::allow_tables_to_appear_in_same_query!(
    user_sessions,
);
