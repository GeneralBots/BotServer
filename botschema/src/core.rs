diesel::table! {
    tenants (id) {
        id -> Uuid,
        name -> Varchar,
        slug -> Varchar,
        created_at -> Timestamptz,
        is_active -> Bool,
        metadata -> Nullable<Jsonb>,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    organizations (org_id) {
        org_id -> Uuid,
        tenant_id -> Uuid,
        name -> Text,
        slug -> Text,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(organizations -> tenants (tenant_id));

diesel::table! {
    organization_invitations (id) {
        id -> Uuid,
        org_id -> Uuid,
        branch_id -> Nullable<Uuid>,
        email -> Varchar,
        role -> Varchar,
        status -> Varchar,
        message -> Nullable<Text>,
        invited_by -> Uuid,
        token -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        expires_at -> Nullable<Timestamptz>,
        accepted_at -> Nullable<Timestamptz>,
        accepted_by -> Nullable<Uuid>,
        groups -> Jsonb,
        invited_by_name -> Nullable<Varchar>,
    }
}

diesel::table! {
    branches (id) {
        id -> Uuid,
        org_id -> Uuid,
        tenant_id -> Uuid,
        slug -> Varchar,
        name -> Varchar,
        description -> Nullable<Text>,
        is_active -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(branches -> organizations (org_id));
diesel::joinable!(branches -> tenants (tenant_id));

diesel::table! {
    bots (id) {
        id -> Uuid,
        org_id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        slug -> Varchar,
        description -> Nullable<Text>,
        llm_provider -> Varchar,
        llm_config -> Jsonb,
        context_provider -> Varchar,
        context_config -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        is_active -> Nullable<Bool>,
        database_name -> Nullable<Varchar>,
        is_public -> Bool,
        is_default_for_branch -> Bool,
    }
}

diesel::joinable!(bots -> branches (branch_id));

diesel::joinable!(bots -> organizations (org_id));

diesel::table! {
    system_automations (id) {
        id -> Uuid,
        bot_id -> Nullable<Uuid>,
        kind -> Int4,
        target -> Nullable<Text>,
        schedule -> Nullable<Text>,
        param -> Text,
        is_active -> Bool,
        last_triggered -> Nullable<Timestamptz>,
        branch_id -> Uuid,
    }
}

diesel::table! {
    // Physical column order follows `6.0.0-01-core` (the migration that
    // actually creates the table; `6.5.15.1-consolidated` uses IF NOT EXISTS
    // and therefore never wins) plus the columns appended by later ALTERs.
    // `6.5.30-user-sessions-schema-fix` retypes `user_id` to VARCHAR, and
    // `6.5.23-branch-scope-cleanup` backfills + enforces `branch_id` NOT NULL.
    user_sessions (id) {
        id -> Uuid,
        user_id -> Varchar,
        bot_id -> Uuid,
        title -> Varchar,
        answer_mode -> Int4,
        context_data -> Jsonb,
        current_tool -> Nullable<Varchar>,
        message_count -> Int4,
        total_tokens -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        last_activity -> Timestamptz,
        tenant_id -> Nullable<Uuid>,
        active_email_account_id -> Nullable<Uuid>,
        branch_id -> Uuid,
        session_id -> Nullable<Varchar>,
        data -> Nullable<Jsonb>,
        expires_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    workflow_executions (id) {
        id -> Uuid,
        bot_id -> Nullable<Uuid>,
        workflow_name -> Text,
        current_step -> Nullable<Int4>,
        state_json -> Nullable<Jsonb>,
        status -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    workflow_events (id) {
        id -> Uuid,
        execution_id -> Uuid,
        workflow_id -> Uuid,
        event_name -> Text,
        event_type -> Text,
        payload -> Jsonb,
        event_data_json -> Nullable<Jsonb>,
        processed -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    bot_shared_memory (id) {
        id -> Uuid,
        source_bot_id -> Uuid,
        target_bot_id -> Uuid,
        memory_key -> Text,
        memory_value -> Text,
        shared_at -> Timestamptz,
    }
}

diesel::table! {
    message_history (id) {
        id -> Uuid,
        session_id -> Uuid,
        user_id -> Uuid,
        role -> Int4,
        content_encrypted -> Text,
        message_type -> Int4,
        media_url -> Nullable<Text>,
        token_count -> Int4,
        processing_time_ms -> Nullable<Int4>,
        llm_model -> Nullable<Varchar>,
        created_at -> Timestamptz,
        message_index -> Int4,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        username -> Text,
        email -> Text,
        password_hash -> Text,
        is_active -> Bool,
        is_admin -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    clicks (id) {
        id -> Uuid,
        campaign_id -> Text,
        email -> Text,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    bot_memories (id) {
        id -> Uuid,
        bot_id -> Nullable<Uuid>,
        key -> Text,
        value -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    basic_tools (id) {
        id -> Uuid,
        bot_id -> Uuid,
        tool_name -> Text,
        file_path -> Text,
        ast_path -> Text,
        mcp_json -> Nullable<Text>,
        tool_json -> Nullable<Text>,
        compiled_at -> Text,
        is_active -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        branch_id -> Uuid,
    }
}

diesel::table! {
    session_tool_associations (id) {
        id -> Text,
        session_id -> Text,
        tool_name -> Text,
        added_at -> Text,
    }
}

diesel::table! {
    bot_configuration (id) {
        id -> Uuid,
        bot_id -> Nullable<Uuid>,
        config_key -> Text,
        config_value -> Text,
        is_encrypted -> Bool,
        config_type -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

mod part2;
pub use part2::*;
