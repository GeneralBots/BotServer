// Canonical definition lives in `botschema` (#1497).
pub use botschema::user_sessions;

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
