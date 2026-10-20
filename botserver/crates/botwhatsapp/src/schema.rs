// @generated automatically by script from migration SQL for issue #707.
// Diesel table definitions for branch-scope cleanup.

// Canonical definition lives in `botschema` (#1497).
pub use botschema::bots;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::bot_configuration;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::user_sessions;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::users;

pub use botschema::message_history;

diesel::allow_tables_to_appear_in_same_query!(user_sessions,);
