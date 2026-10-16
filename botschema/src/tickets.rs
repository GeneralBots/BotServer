// use crate::core::{branches};

diesel::table! {
    support_tickets (id) {
        id -> Uuid,
        branch_id -> Uuid,
        ticket_number -> Varchar,
        subject -> Varchar,
        description -> Nullable<Text>,
        status -> Varchar,
        priority -> Varchar,
        category -> Nullable<Varchar>,
        source -> Varchar,
        requester_id -> Nullable<Uuid>,
        requester_email -> Nullable<Varchar>,
        requester_name -> Nullable<Varchar>,
        assignee_id -> Nullable<Uuid>,
        team_id -> Nullable<Uuid>,
        due_date -> Nullable<Timestamptz>,
        first_response_at -> Nullable<Timestamptz>,
        resolved_at -> Nullable<Timestamptz>,
        closed_at -> Nullable<Timestamptz>,
        satisfaction_rating -> Nullable<Int4>,
        record_type -> Varchar,
        tags -> Array<Text>,
        custom_fields -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    ticket_comments (id) {
        id -> Uuid,
        ticket_id -> Uuid,
        author_id -> Nullable<Uuid>,
        author_name -> Nullable<Varchar>,
        author_email -> Nullable<Varchar>,
        content -> Text,
        is_internal -> Bool,
        attachments -> Jsonb,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    ticket_sla_policies (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        priority -> Varchar,
        first_response_hours -> Int4,
        resolution_hours -> Int4,
        business_hours_only -> Bool,
        is_active -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    ticket_canned_responses (id) {
        id -> Uuid,
        branch_id -> Uuid,
        title -> Varchar,
        content -> Text,
        category -> Nullable<Varchar>,
        shortcut -> Nullable<Varchar>,
        created_by -> Nullable<Uuid>,
        is_active -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    ticket_categories (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        description -> Nullable<Text>,
        parent_id -> Nullable<Uuid>,
        color -> Nullable<Varchar>,
        icon -> Nullable<Varchar>,
        sort_order -> Int4,
        is_active -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    ticket_tags (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        color -> Nullable<Varchar>,
        created_at -> Timestamptz,
    }
}
