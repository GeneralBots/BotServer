
pub use botlib::db_pool::DbPool;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::user_sessions;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::message_history;
// Canonical definition lives in `botschema` (#1497).
pub use botschema::users;
// Canonical definition lives in `botschema` (#1497).
pub use botschema::bots;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::bot_configuration;

// `message_history`, `users` and `bot_configuration` are re-exported from
// `botschema`; their join rules live there (#1497) — a cross-table join between
// two foreign types would violate the orphan rule.
diesel::allow_tables_to_appear_in_same_query!(user_sessions,);
