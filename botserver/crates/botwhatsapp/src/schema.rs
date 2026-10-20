// @generated automatically by script from migration SQL for issue #707.
// Diesel table definitions for branch-scope cleanup.

// Canonical definition lives in `botschema` (#1497).
pub use botschema::bots;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::bot_configuration;

diesel::table! {
    user_sessions (id) {
        id -> Uuid,
        user_id -> Varchar,
        bot_id -> Uuid,
        title -> Varchar,
        answer_mode -> Integer,
        context_data -> Jsonb,
        current_tool -> Nullable<Varchar>,
        message_count -> Int4,
        total_tokens -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        last_activity -> Timestamptz,
        tenant_id -> Nullable<Uuid>,
        active_email_account_id -> Nullable<Uuid>,
        branch_id -> Nullable<Uuid>,
        session_id -> Nullable<Varchar>,
        data -> Nullable<Jsonb>,
        expires_at -> Nullable<Timestamptz>,
    }
}

// Canonical definition lives in `botschema` (#1497).
pub use botschema::users;

pub use botschema::message_history;

diesel::allow_tables_to_appear_in_same_query!(user_sessions,);
