//! Split from `people.rs` — marketing, warmup and IP-routing tables (#1497).

use super::*;

diesel::table! {
    marketing_campaign_events (id) {
        id -> Uuid,
        branch_id -> Uuid,
        campaign_id -> Uuid,
        channel -> Nullable<Varchar>,
        event_type -> Varchar,
        recipient_email -> Nullable<Varchar>,
        status -> Nullable<Varchar>,
        error_message -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    marketing_contacts (id) {
        id -> Uuid,
        branch_id -> Uuid,
        list_id -> Nullable<Uuid>,
        email -> Varchar,
        name -> Nullable<Varchar>,
        phone -> Nullable<Varchar>,
        metadata -> Nullable<Jsonb>,
        subscribed -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    advisor_recommendations (id) {

    id -> Uuid,
    branch_id -> Uuid,
    campaign_id -> Nullable<Uuid>,
    recommendation -> Text,
    reason -> Nullable<Text>,
    #[max_length = 100]
    check_name -> Varchar,
    #[max_length = 20]
    severity -> Varchar,
    message -> Text,
    details -> Nullable<Text>,
    dismissed -> Bool,
    created_at -> Timestamptz,
    }
}


diesel::table! {
    campaign_metrics (id) {
        id -> Uuid,
        branch_id -> Uuid,
        campaign_id -> Nullable<Uuid>,
        metric_type -> Varchar,
        metric_value -> Numeric,
        recorded_at -> Timestamptz,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    warmup_schedules (id) {
        id -> Uuid,
        branch_id -> Uuid,
        org_id -> Uuid,
        email -> Varchar,
        ip -> Varchar,
        daily_limit -> Nullable<Int4>,
        current_count -> Nullable<Int4>,
        current_day -> Nullable<Int4>,
        started_at -> Nullable<Timestamptz>,
        status -> Nullable<Varchar>,
        paused_reason -> Nullable<Text>,
        is_active -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    marketing_email_opens (id) {
        id -> Uuid,
        branch_id -> Uuid,
        campaign_id -> Nullable<Uuid>,
        email -> Varchar,
        opened_at -> Timestamptz,
        ip_address -> Nullable<Varchar>,
        user_agent -> Nullable<Text>,
    }
}

diesel::table! {
    ip_reputation (id) {
        id -> Uuid,
        org_id -> Uuid,
        ip -> Varchar,
        provider -> Varchar,
        delivered -> Int8,
        bounced -> Int8,
        complained -> Int8,
        window_start -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    ip_rotations (id) {
        id -> Uuid,
        org_id -> Uuid,
        ip_address -> Varchar,
        strategy -> Nullable<Varchar>,
        current_index -> Nullable<Int4>,
        last_used -> Nullable<Timestamptz>,
        is_active -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    org_ips (id) {
        id -> Uuid,
        org_id -> Uuid,
        ip_address -> Varchar,
        provider -> Nullable<Varchar>,
        is_active -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    crm_contacts,
    marketing_campaigns,
    marketing_campaign_events,
    marketing_lists,
    marketing_templates,
    marketing_contacts,
    email_tracking,
    marketing_recipients,
    advisor_recommendations,
    campaign_metrics,
    warmup_schedules,
    marketing_email_opens,
    ip_reputation,
    ip_rotations,
    org_ips,
);
