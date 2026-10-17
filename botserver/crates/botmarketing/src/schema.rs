// @generated automatically from migration SQL for issue #707.
// Diesel table definitions for branch-scope cleanup.

// Canonical definition lives in `botschema` (#1497).
pub use botschema::bots;

diesel::table! {
    system_automations (id) {
        id -> Uuid,
        branch_id -> Uuid,
        bot_id -> Uuid,
        name -> Varchar,
        kind -> Int4,
        event_type -> Varchar,
        action_type -> Varchar,
        config -> Nullable<Jsonb>,
        is_active -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    crm_contacts (id) {
        id -> Uuid,
        branch_id -> Uuid,
        first_name -> Nullable<Varchar>,
        last_name -> Nullable<Varchar>,
        email -> Nullable<Varchar>,
        phone -> Nullable<Varchar>,
        mobile -> Nullable<Varchar>,
        company -> Nullable<Varchar>,
        job_title -> Nullable<Varchar>,
        source -> Nullable<Varchar>,
        status -> Nullable<Varchar>,
        tags -> Array<Text>,
        custom_fields -> Nullable<Jsonb>,
        address_street -> Nullable<Varchar>,
        address_city -> Nullable<Varchar>,
        address_state -> Nullable<Varchar>,
        address_country -> Nullable<Varchar>,
        address_zip -> Nullable<Varchar>,
        notes -> Nullable<Text>,
        owner_id -> Nullable<Uuid>,
        pass_hash -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    marketing_campaigns (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        campaign_type -> Varchar,
        status -> Nullable<Varchar>,
        starts_at -> Nullable<Timestamptz>,
        ends_at -> Nullable<Timestamptz>,
        budget -> Nullable<Numeric>,
        metrics -> Nullable<Jsonb>,
        run_offset -> Nullable<Int4>,
        pause_requested -> Nullable<Bool>,
        stop_requested -> Nullable<Bool>,
        started_at -> Nullable<Timestamptz>,
        completed_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

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
    marketing_lists (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        list_type -> Varchar,
        description -> Nullable<Text>,
        query_text -> Nullable<Text>,
        member_count -> Nullable<Int4>,
        contact_count -> Nullable<Int4>,
        is_dynamic -> Nullable<Bool>,
        criteria -> Nullable<Jsonb>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    marketing_templates (id) {
        id -> Uuid,
        org_id -> Uuid,
        bot_id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        channel -> Varchar,
        subject -> Nullable<Varchar>,
        body -> Nullable<Text>,
        variables -> Nullable<Jsonb>,
        media_url -> Nullable<Varchar>,
        ai_prompt -> Nullable<Text>,
        approved -> Nullable<Bool>,
        meta_template_id -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
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
    email_tracking (id) {
        id -> Uuid,
        branch_id -> Uuid,
        campaign_id -> Nullable<Uuid>,
        recipient_id -> Nullable<Uuid>,
        email -> Varchar,
        event_type -> Varchar,
        open_token -> Nullable<Varchar>,
        open_tracking_enabled -> Nullable<Bool>,
        opened -> Nullable<Bool>,
        clicked -> Nullable<Bool>,
        message_id -> Nullable<Varchar>,
        opened_at -> Nullable<Timestamptz>,
        clicked_at -> Nullable<Timestamptz>,
        metadata -> Nullable<Jsonb>,
        ip_address -> Nullable<Varchar>,
        user_agent -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    marketing_recipients (id) {
        id -> Uuid,
        branch_id -> Uuid,
        campaign_id -> Nullable<Uuid>,
        list_id -> Nullable<Uuid>,
        contact_id -> Nullable<Uuid>,
        email -> Varchar,
        name -> Nullable<Varchar>,
        status -> Nullable<Varchar>,
        channel -> Nullable<Varchar>,
        sent_at -> Nullable<Timestamptz>,
        failed_at -> Nullable<Timestamptz>,
        delivered_at -> Nullable<Timestamptz>,
        opened_at -> Nullable<Timestamptz>,
        clicked_at -> Nullable<Timestamptz>,
        error_message -> Nullable<Text>,
        response -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    advisor_recommendations (id) {
        id -> Uuid,
        branch_id -> Uuid,
        campaign_id -> Nullable<Uuid>,
        recommendation -> Text,
        reason -> Nullable<Text>,
        check_name -> Varchar,
        severity -> Varchar,
        message -> Text,
        details -> Nullable<Text>,
        dismissed -> Nullable<Bool>,
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
    system_automations,
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
