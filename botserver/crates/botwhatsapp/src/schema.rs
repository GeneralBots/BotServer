// @generated automatically by script from migration SQL for issue #707.
// Diesel table definitions for branch-scope cleanup.

// Canonical definition lives in `botschema` (#1497).
pub use botschema::bots;

diesel::table! {
    bot_configuration (id) {
        id -> Uuid,
        branch_id -> Uuid,
        bot_id -> Uuid,
        config_key -> Varchar,
        config_value -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

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

diesel::table! {
    users (id) {
        id -> Uuid,
        username -> Varchar,
        email -> Varchar,
        password_hash -> Varchar,
        phone_number -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        is_active -> Nullable<Bool>,
    }
}

pub use botschema::message_history;

diesel::allow_tables_to_appear_in_same_query!(
    bot_configuration,
    user_sessions,
    users,
);
