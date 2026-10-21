// use crate::core::{branches};

diesel::table! {
    support_tickets (id) {

    id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 50]
    ticket_number -> Varchar,
    #[max_length = 500]
    subject -> Varchar,
    description -> Nullable<Text>,
    #[max_length = 50]
    status -> Varchar,
    #[max_length = 50]
    priority -> Varchar,
    #[max_length = 100]
    category -> Nullable<Varchar>,
    #[max_length = 50]
    source -> Varchar,
    requester_id -> Nullable<Uuid>,
    #[max_length = 255]
    requester_email -> Nullable<Varchar>,
    #[max_length = 255]
    requester_name -> Nullable<Varchar>,
    assignee_id -> Nullable<Uuid>,
    team_id -> Nullable<Uuid>,
    due_date -> Nullable<Timestamptz>,
    first_response_at -> Nullable<Timestamptz>,
    resolved_at -> Nullable<Timestamptz>,
    closed_at -> Nullable<Timestamptz>,
    satisfaction_rating -> Nullable<Int4>,
    tags -> Array<Nullable<Text>>,
    custom_fields -> Jsonb,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    branch_id -> Nullable<Uuid>,
    #[max_length = 50]
    record_type -> Varchar,
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
    org_id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 255]
    name -> Varchar,
    #[max_length = 50]
    priority -> Varchar,
    first_response_hours -> Int4,
    resolution_hours -> Int4,
    business_hours_only -> Bool,
    is_active -> Bool,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    branch_id -> Nullable<Uuid>,
    }
}


diesel::table! {
    ticket_canned_responses (id) {

    id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 255]
    title -> Varchar,
    content -> Text,
    #[max_length = 100]
    category -> Nullable<Varchar>,
    #[max_length = 50]
    shortcut -> Nullable<Varchar>,
    created_by -> Nullable<Uuid>,
    is_active -> Bool,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    branch_id -> Nullable<Uuid>,
    }
}


diesel::table! {
    ticket_categories (id) {

    id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 255]
    name -> Varchar,
    description -> Nullable<Text>,
    parent_id -> Nullable<Uuid>,
    #[max_length = 20]
    color -> Nullable<Varchar>,
    #[max_length = 50]
    icon -> Nullable<Varchar>,
    sort_order -> Int4,
    is_active -> Bool,
    created_at -> Timestamptz,
    branch_id -> Nullable<Uuid>,
    }
}


diesel::table! {
    ticket_tags (id) {

    id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 100]
    name -> Varchar,
    #[max_length = 20]
    color -> Nullable<Varchar>,
    created_at -> Timestamptz,
    branch_id -> Nullable<Uuid>,
    }
}

