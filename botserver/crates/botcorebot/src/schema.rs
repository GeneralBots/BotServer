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
diesel::table! {
    bot_configuration (id) {
        id -> Uuid,
        branch_id -> Uuid,
        bot_id -> Uuid,
        config_key -> Varchar,
        config_value -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        is_encrypted -> Bool,
        config_type -> Text,
    }
}

diesel::table! {
    system_automations (id) {
        id -> Uuid,
        branch_id -> Uuid,
        bot_id -> Uuid,
        name -> Varchar,
        event_type -> Varchar,
        action_type -> Varchar,
        config -> Nullable<Jsonb>,
        is_active -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        kind -> Int4,
        target -> Nullable<Text>,
        schedule -> Nullable<Text>,
        param -> Text,
        last_triggered -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    workflow_executions (id) {
        id -> Uuid,
        branch_id -> Uuid,
        bot_id -> Uuid,
        workflow_id -> Uuid,
        status -> Varchar,
        input -> Nullable<Jsonb>,
        output -> Nullable<Jsonb>,
        started_at -> Nullable<Timestamptz>,
        completed_at -> Nullable<Timestamptz>,
        error -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        workflow_name -> Text,
        current_step -> Nullable<Int4>,
        state_json -> Nullable<Jsonb>,
    }
}

diesel::table! {
    workflow_events (id) {
        id -> Uuid,
        workflow_id -> Nullable<Uuid>,
        event_name -> Text,
        event_data_json -> Nullable<Text>,
        processed -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    organizations (org_id) {
        org_id -> Uuid,
        tenant_id -> Uuid,
        name -> Varchar,
        slug -> Varchar,
        created_at -> Timestamptz,
    }
}

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
