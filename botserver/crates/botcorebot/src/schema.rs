// Canonical definition lives in `botschema` (#1497).
pub use botschema::bots;

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

// Canonical definition lives in `botschema` (#1497).
pub use botschema::message_history;
// Canonical definition lives in `botschema` (#1497).
pub use botschema::users;
pub use botschema::bot_memories;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::bot_shared_memory;
// Canonical definition lives in `botschema` (#1497).
pub use botschema::basic_tools;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::session_tool_associations;
// Canonical definition lives in `botschema` (#1497).
pub use botschema::bot_configuration;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::system_automations;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::workflow_executions;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::workflow_events;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::organizations;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::tenants;

pub use botschema::clicks;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::organization_invitations;

pub use botschema::user_organizations;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::user_preferences;

pub use botschema::user_login_tokens;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::website_crawls;

// `message_history`, `users`, `bot_shared_memory` and `session_tool_associations`
// are re-exported from `botschema`; their join rules live there (#1497) — the
// `allow_tables_to_appear_in_same_query!` list moved to `botschema::core`, since
// every pair here is between two foreign types (orphan rule).
