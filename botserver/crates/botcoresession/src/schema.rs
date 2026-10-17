
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
diesel::table! {
    bots (id) {
        id -> Uuid,
        branch_id -> Uuid,
        bot_id -> Uuid,
        name -> Varchar,
        slug -> Varchar,
        org_id -> Uuid,
        tenant_id -> Nullable<Uuid>,
        is_default_for_branch -> Nullable<Bool>,
        description -> Nullable<Text>,
        is_public -> Nullable<Bool>,
        is_active -> Nullable<Bool>,
        avatar_url -> Nullable<Varchar>,
        settings -> Nullable<Jsonb>,
        metadata -> Nullable<Jsonb>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        llm_provider -> Varchar,
        llm_config -> Jsonb,
        context_provider -> Varchar,
        context_config -> Jsonb,
        database_name -> Nullable<Varchar>,
    }
}

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

// `message_history` and `users` are re-exported from `botschema`; their join
// rules live there (#1497).
diesel::allow_tables_to_appear_in_same_query!(
    user_sessions,
    bots,
    bot_configuration,
);
