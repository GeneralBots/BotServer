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
        title -> Text,
        context_data -> Jsonb,
        current_tool -> Nullable<Text>,
    }
}

pub use botschema::message_history;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::bots;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::users;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::attendance_webhooks;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::attendance_sla_policies;

// Canonical definition lives in `botschema` (#1497): the database also has
// `met_at`, a nullable `status` and a nullable `created_at`.
pub use botschema::attendance_sla_events;
