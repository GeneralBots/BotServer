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

diesel::table! {
    message_history (id) {
        id -> Uuid,
        session_id -> Uuid,
        user_id -> Uuid,
        role -> Int4,
        content_encrypted -> Text,
        message_type -> Int4,
        message_index -> Int4,
        created_at -> Timestamptz,
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
    bot_memories (id) {
        id -> Uuid,
        branch_id -> Uuid,
        bot_id -> Uuid,
        key -> Varchar,
        value -> Text,
        #[sql_name = "type"]
        memory_type -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
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
    basic_tools (id) {
        id -> Uuid,
        branch_id -> Uuid,
        bot_id -> Uuid,
        name -> Varchar,
        source -> Nullable<Text>,
        compiled -> Nullable<Text>,
        version -> Int4,
        is_active -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        tool_name -> Text,
        file_path -> Text,
        ast_path -> Text,
        file_hash -> Text,
        mcp_json -> Nullable<Text>,
        tool_json -> Nullable<Text>,
        compiled_at -> Text,
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

diesel::table! {
    tenants (id) {
        id -> Uuid,
        name -> Varchar,
        slug -> Varchar,
        created_at -> Timestamptz,
    }
}

pub use botschema::clicks;

diesel::table! {
    organization_invitations (id) {
        id -> Uuid,
        org_id -> Uuid,
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
    }
}

pub use botschema::user_organizations;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::user_preferences;

pub use botschema::user_login_tokens;

diesel::table! {
    website_crawls (id) {
        id -> Uuid,
        branch_id -> Uuid,
        bot_id -> Uuid,
        url -> Text,
        status -> Nullable<Varchar>,
        depth -> Nullable<Int4>,
        content -> Nullable<Text>,
        error -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        last_crawled -> Nullable<Timestamptz>,
        next_crawl -> Nullable<Timestamptz>,
        expires_policy -> Varchar,
        max_depth -> Nullable<Int4>,
        max_pages -> Nullable<Int4>,
        crawl_status -> Nullable<Int2>,
        pages_crawled -> Nullable<Int4>,
        error_message -> Nullable<Text>,
        refresh_policy -> Nullable<Varchar>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    bots,
    user_sessions,
    message_history,
    users,
    bot_memories,
    bot_shared_memory,
    basic_tools,
    session_tool_associations,
    bot_configuration,
    system_automations,
    workflow_executions,
    workflow_events,
    organizations,
    tenants,
    organization_invitations,
    website_crawls,
);
