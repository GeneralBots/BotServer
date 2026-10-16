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
    users (id) {
        id -> Uuid,
        username -> Varchar,
        email -> Varchar,
    }
}

diesel::table! {
    attendance_webhooks (id) {
        id -> Uuid,
        branch_id -> Uuid,
        url -> Text,
        event_types -> Nullable<Text>,
        is_active -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        webhook_url -> Varchar,
        events -> Nullable<Text>,
        secret_key -> Nullable<Text>,
    }
}

diesel::table! {
    attendance_sla_policies (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        response_time_minutes -> Int4,
        resolution_time_minutes -> Nullable<Int4>,
        priority -> Nullable<Int4>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        channel -> Nullable<Varchar>,
        first_response_minutes -> Nullable<Int4>,
        resolution_minutes -> Nullable<Int4>,
        escalate_on_breach -> Nullable<Bool>,
        is_active -> Nullable<Bool>,
    }
}

diesel::table! {
    attendance_sla_events (id) {
        id -> Uuid,
        session_id -> Uuid,
        sla_policy_id -> Uuid,
        event_type -> Varchar,
        due_at -> Timestamptz,
        status -> Varchar,
        breached_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}
