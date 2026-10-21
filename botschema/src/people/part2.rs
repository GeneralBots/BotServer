//! Split from `people.rs` — 248 lines.

use super::*;

diesel::table! {
    people_skills (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        category -> Nullable<Varchar>,
        description -> Nullable<Text>,
        is_active -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    people_person_skills (id) {
        id -> Uuid,
        person_id -> Uuid,
        skill_id -> Uuid,
        proficiency_level -> Int4,
        years_experience -> Nullable<Numeric>,
        verified_by -> Nullable<Uuid>,
        verified_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    people_time_off (id) {
        id -> Uuid,
        branch_id -> Uuid,
        person_id -> Uuid,
        time_off_type -> Varchar,
        status -> Varchar,
        start_date -> Date,
        end_date -> Date,
        hours_requested -> Nullable<Numeric>,
        reason -> Nullable<Text>,
        approved_by -> Nullable<Uuid>,
        approved_at -> Nullable<Timestamptz>,
        notes -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(people -> branches (branch_id));
diesel::joinable!(people_teams -> branches (branch_id));
diesel::joinable!(people_team_members -> people_teams (team_id));
diesel::joinable!(people_team_members -> people (person_id));
diesel::joinable!(people_org_chart -> branches (branch_id));
diesel::joinable!(people_departments -> branches (branch_id));
diesel::joinable!(people_skills -> branches (branch_id));
diesel::joinable!(people_person_skills -> people (person_id));
diesel::joinable!(people_person_skills -> people_skills (skill_id));
diesel::joinable!(people_time_off -> branches (branch_id));
diesel::joinable!(crm_deals -> people_departments (department_id));
diesel::joinable!(attendance_sla_events -> attendance_sla_policies (sla_policy_id));

diesel::table! {
    crm_deals (id) {
        id -> Uuid,
        branch_id -> Uuid,
        contact_id -> Nullable<Uuid>,
        account_id -> Nullable<Uuid>,
        am_id -> Nullable<Uuid>,
        owner_id -> Nullable<Uuid>,
        lead_id -> Nullable<Uuid>,
        title -> Nullable<Varchar>,
        name -> Varchar,
        description -> Nullable<Text>,
        value -> Nullable<Float8>,
        currency -> Nullable<Varchar>,
        stage_id -> Nullable<Uuid>,
        stage -> Nullable<Varchar>,
        probability -> Nullable<Int4>,
        source -> Nullable<Varchar>,
        segment_id -> Nullable<Uuid>,
        department_id -> Nullable<Uuid>,
        expected_close_date -> Nullable<Date>,
        actual_close_date -> Nullable<Date>,
        period -> Nullable<Int4>,
        deal_date -> Nullable<Date>,
        closed_at -> Nullable<Timestamptz>,
        lost_reason -> Nullable<Text>,
        won -> Nullable<Bool>,
        notes -> Nullable<Text>,
        tags -> Nullable<Array<Text>>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        custom_fields -> Jsonb,
        org_id -> Uuid,
        bot_id -> Uuid,
    }
}

diesel::table! {
    crm_deal_segments (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        description -> Nullable<Varchar>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    marketing_campaigns (id) {
        id -> Uuid,
        branch_id -> Uuid,
        deal_id -> Nullable<Uuid>,
        name -> Varchar,
        status -> Nullable<Varchar>,
        content_template -> Jsonb,
        sent_at -> Nullable<Timestamptz>,
        completed_at -> Nullable<Timestamptz>,
        metrics -> Nullable<Jsonb>,
        budget -> Nullable<Numeric>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        campaign_type -> Varchar,
        starts_at -> Nullable<Timestamptz>,
        ends_at -> Nullable<Timestamptz>,
        run_offset -> Nullable<Int4>,
        pause_requested -> Nullable<Bool>,
        stop_requested -> Nullable<Bool>,
        started_at -> Nullable<Timestamptz>,
        org_id -> Nullable<Uuid>,
        bot_id -> Nullable<Uuid>,
    }
}

diesel::table! {
    marketing_lists (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        list_type -> Varchar,
        query_text -> Nullable<Text>,
        contact_count -> Nullable<Int4>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        description -> Nullable<Text>,
        member_count -> Nullable<Int4>,
        is_dynamic -> Nullable<Bool>,
        criteria -> Nullable<Jsonb>,
        org_id -> Nullable<Uuid>,
        bot_id -> Nullable<Uuid>,
    }
}

diesel::table! {
    marketing_list_contacts (list_id, contact_id) {
        list_id -> Uuid,
        contact_id -> Uuid,
        added_at -> Timestamptz,
    }
}

diesel::table! {
    marketing_recipients (id) {
        id -> Uuid,
        campaign_id -> Nullable<Uuid>,
        contact_id -> Nullable<Uuid>,
        deal_id -> Nullable<Uuid>,
        channel -> Varchar,
        status -> Varchar,
        sent_at -> Nullable<Timestamptz>,
        delivered_at -> Nullable<Timestamptz>,
        failed_at -> Nullable<Timestamptz>,
        error_message -> Nullable<Text>,
        response -> Nullable<Text>,
        created_at -> Nullable<Timestamptz>,
        branch_id -> Uuid,
        list_id -> Nullable<Uuid>,
        email -> Varchar,
        name -> Nullable<Varchar>,
        opened_at -> Nullable<Timestamptz>,
        clicked_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    marketing_templates (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        channel -> Varchar,
        subject -> Nullable<Varchar>,
        body -> Nullable<Text>,
        media_url -> Nullable<Varchar>,
        ai_prompt -> Nullable<Text>,
        variables -> Nullable<Jsonb>,
        approved -> Nullable<Bool>,
        meta_template_id -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        org_id -> Uuid,
        bot_id -> Uuid,
    }
}

diesel::table! {
    email_tracking (id) {
        id -> Uuid,
        recipient_id -> Nullable<Uuid>,
        campaign_id -> Nullable<Uuid>,
        message_id -> Nullable<Varchar>,
        open_token -> Nullable<Uuid>,
        open_tracking_enabled -> Nullable<Bool>,
        opened -> Nullable<Bool>,
        opened_at -> Nullable<Timestamptz>,
        clicked -> Nullable<Bool>,
        clicked_at -> Nullable<Timestamptz>,
        ip_address -> Nullable<Varchar>,
        user_agent -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    whatsapp_business (id) {
        id -> Uuid,
        phone_number_id -> Nullable<Varchar>,
        business_account_id -> Nullable<Varchar>,
        access_token -> Nullable<Text>,
        webhooks_verified -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    attendance_sla_policies (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        channel -> Nullable<Varchar>,
        priority -> Nullable<Varchar>,
        first_response_minutes -> Nullable<Int4>,
        resolution_minutes -> Nullable<Int4>,
        escalate_on_breach -> Nullable<Bool>,
        is_active -> Nullable<Bool>,
        created_at -> Timestamptz,
        org_id -> Nullable<Uuid>,
        bot_id -> Nullable<Uuid>,
    }
}

diesel::table! {
    attendance_sla_events (id) {
        id -> Uuid,
        session_id -> Uuid,
        sla_policy_id -> Uuid,
        event_type -> Varchar,
        due_at -> Timestamptz,
        met_at -> Nullable<Timestamptz>,
        breached_at -> Nullable<Timestamptz>,
        status -> Nullable<Varchar>,
        created_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    attendance_webhooks (id) {
        id -> Uuid,
        branch_id -> Uuid,
        webhook_url -> Varchar,
        events -> Nullable<Text>,
        is_active -> Nullable<Bool>,
        secret_key -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        org_id -> Nullable<Uuid>,
    }
}

// HR / time-clock tables (canonical, #1497).

diesel::table! {
    hr_clock_entries (id) {

    id -> Uuid,
    bot_id -> Uuid,
    person_id -> Uuid,
    #[max_length = 20]
    entry_type -> Varchar,
    timestamp -> Timestamptz,
    latitude -> Nullable<Numeric>,
    longitude -> Nullable<Numeric>,
    geofence_id -> Nullable<Uuid>,
    #[max_length = 100]
    device_id -> Nullable<Varchar>,
    #[max_length = 64]
    ip_address -> Nullable<Varchar>,
    notes -> Nullable<Text>,
    created_at -> Timestamptz,
    }
}

diesel::table! {
    hr_work_periods (id) {

    id -> Uuid,
    bot_id -> Uuid,
    person_id -> Uuid,
    period_start -> Date,
    period_end -> Date,
    hours_worked -> Numeric,
    hours_overtime -> Numeric,
    hours_night -> Numeric,
    absences -> Int4,
    #[max_length = 30]
    status -> Varchar,
    created_at -> Timestamptz,
    }
}

diesel::table! {
    hr_overtime_rules (id) {

    id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 100]
    name -> Varchar,
    weekly_threshold_hours -> Numeric,
    daily_threshold_hours -> Nullable<Numeric>,
    overtime_multiplier -> Numeric,
    night_shift_multiplier -> Numeric,
    holiday_multiplier -> Numeric,
    is_active -> Bool,
    created_at -> Timestamptz,
    }
}

diesel::table! {
    hr_schedules (id) {

    id -> Uuid,
    bot_id -> Uuid,
    person_id -> Uuid,
    weekday -> Int4,
    start_time -> Time,
    end_time -> Time,
    break_minutes -> Int4,
    effective_from -> Date,
    effective_until -> Nullable<Date>,
    created_at -> Timestamptz,
    }
}

diesel::table! {
    hr_holidays (id) {

    id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 100]
    name -> Varchar,
    holiday_date -> Date,
    is_recurring -> Bool,
    #[max_length = 50]
    region -> Nullable<Varchar>,
    created_at -> Timestamptz,
    }
}
