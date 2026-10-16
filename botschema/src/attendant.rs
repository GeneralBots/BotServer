use crate::core::{branches};

diesel::table! {
    attendant_queues (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        description -> Nullable<Text>,
        priority -> Int4,
        max_wait_minutes -> Int4,
        auto_assign -> Bool,
        working_hours -> Jsonb,
        is_active -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    attendant_sessions (id) {
        id -> Uuid,
        branch_id -> Uuid,
        session_number -> Varchar,
        channel -> Varchar,
        customer_id -> Nullable<Uuid>,
        customer_name -> Nullable<Varchar>,
        customer_email -> Nullable<Varchar>,
        customer_phone -> Nullable<Varchar>,
        status -> Varchar,
        priority -> Int4,
        agent_id -> Nullable<Uuid>,
        queue_id -> Nullable<Uuid>,
        subject -> Nullable<Varchar>,
        initial_message -> Nullable<Text>,
        started_at -> Timestamptz,
        assigned_at -> Nullable<Timestamptz>,
        first_response_at -> Nullable<Timestamptz>,
        ended_at -> Nullable<Timestamptz>,
        wait_time_seconds -> Nullable<Int4>,
        handle_time_seconds -> Nullable<Int4>,
        satisfaction_rating -> Nullable<Int4>,
        satisfaction_comment -> Nullable<Text>,
        tags -> Array<Text>,
        metadata -> Jsonb,
        notes -> Nullable<Text>,
        transfer_count -> Int4,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    attendant_session_messages (id) {
        id -> Uuid,
        session_id -> Uuid,
        sender_type -> Varchar,
        sender_id -> Nullable<Uuid>,
        sender_name -> Nullable<Varchar>,
        content -> Text,
        content_type -> Varchar,
        attachments -> Jsonb,
        is_internal -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    attendant_queue_agents (id) {
        id -> Uuid,
        queue_id -> Uuid,
        agent_id -> Uuid,
        max_concurrent -> Int4,
        priority -> Int4,
        skills -> Array<Text>,
        is_active -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    attendant_agent_status (id) {
        id -> Uuid,
        branch_id -> Uuid,
        agent_id -> Uuid,
        status -> Varchar,
        status_message -> Nullable<Varchar>,
        current_sessions -> Int4,
        max_sessions -> Int4,
        last_activity_at -> Timestamptz,
        break_started_at -> Nullable<Timestamptz>,
        break_reason -> Nullable<Varchar>,
        available_since -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    attendant_transfers (id) {
        id -> Uuid,
        session_id -> Uuid,
        from_agent_id -> Nullable<Uuid>,
        to_agent_id -> Nullable<Uuid>,
        to_queue_id -> Nullable<Uuid>,
        reason -> Nullable<Varchar>,
        notes -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    attendant_canned_responses (id) {
        id -> Uuid,
        branch_id -> Uuid,
        title -> Varchar,
        content -> Text,
        shortcut -> Nullable<Varchar>,
        category -> Nullable<Varchar>,
        queue_id -> Nullable<Uuid>,
        is_active -> Bool,
        usage_count -> Int4,
        created_by -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    attendant_tags (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        color -> Nullable<Varchar>,
        description -> Nullable<Text>,
        is_active -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    attendant_wrap_up_codes (id) {
        id -> Uuid,
        branch_id -> Uuid,
        code -> Varchar,
        name -> Varchar,
        description -> Nullable<Text>,
        requires_notes -> Bool,
        is_active -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    attendant_session_wrap_up (id) {
        id -> Uuid,
        session_id -> Uuid,
        wrap_up_code_id -> Nullable<Uuid>,
        notes -> Nullable<Text>,
        follow_up_required -> Bool,
        follow_up_date -> Nullable<Date>,
        created_by -> Nullable<Uuid>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    attendant_attachments (id) {
        id -> Uuid,
        session_id -> Uuid,
        name -> Varchar,
        content_type -> Varchar,
        size_bytes -> BigInt,
        data -> Bytea,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(attendant_queues -> branches (branch_id));
diesel::joinable!(attendant_attachments -> attendant_sessions (session_id));
diesel::joinable!(attendant_sessions -> branches (branch_id));
diesel::joinable!(attendant_sessions -> attendant_queues (queue_id));
diesel::joinable!(attendant_session_messages -> attendant_sessions (session_id));
diesel::joinable!(attendant_queue_agents -> attendant_queues (queue_id));
diesel::joinable!(attendant_agent_status -> branches (branch_id));
diesel::joinable!(attendant_transfers -> attendant_sessions (session_id));
diesel::joinable!(attendant_canned_responses -> branches (branch_id));
diesel::joinable!(attendant_tags -> branches (branch_id));
diesel::joinable!(attendant_wrap_up_codes -> branches (branch_id));
diesel::joinable!(attendant_session_wrap_up -> attendant_sessions (session_id));
diesel::joinable!(attendant_session_wrap_up -> attendant_wrap_up_codes (wrap_up_code_id));
