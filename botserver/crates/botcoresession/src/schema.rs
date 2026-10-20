
pub use botlib::db_pool::DbPool;

diesel::table! {
    user_sessions (id) {
        id -> Uuid,
        branch_id -> Uuid,
        bot_id -> Uuid,
        session_id -> Varchar,
        user_id -> Nullable<Varchar>,
        data -> Nullable<Jsonb>,
        expires_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

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
