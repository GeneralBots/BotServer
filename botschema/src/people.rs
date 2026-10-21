use crate::core::{branches};

diesel::table! {
    // Physical order: `org_id`/`bot_id` first (tenant scope), address columns
    // before `notes`, `branch_id` and `pass_hash` last. `status` is NOT NULL.
    crm_contacts (id) {
        id -> Uuid,
        org_id -> Uuid,
        bot_id -> Uuid,
        first_name -> Nullable<Varchar>,
        last_name -> Nullable<Varchar>,
        email -> Nullable<Varchar>,
        phone -> Nullable<Varchar>,
        mobile -> Nullable<Varchar>,
        company -> Nullable<Varchar>,
        job_title -> Nullable<Varchar>,
        source -> Nullable<Varchar>,
        status -> Varchar,
        tags -> Nullable<Array<Nullable<Text>>>,
        custom_fields -> Nullable<Jsonb>,
        address_line1 -> Nullable<Varchar>,
        address_line2 -> Nullable<Varchar>,
        city -> Nullable<Varchar>,
        state -> Nullable<Varchar>,
        postal_code -> Nullable<Varchar>,
        country -> Nullable<Varchar>,
        notes -> Nullable<Text>,
        owner_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        branch_id -> Uuid,
        pass_hash -> Nullable<Varchar>,
    }
}

diesel::table! {
    crm_accounts (id) {

    id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 255]
    name -> Varchar,
    #[max_length = 500]
    website -> Nullable<Varchar>,
    #[max_length = 100]
    industry -> Nullable<Varchar>,
    employees_count -> Nullable<Int4>,
    annual_revenue -> Nullable<Float8>,
    #[max_length = 50]
    phone -> Nullable<Varchar>,
    #[max_length = 255]
    email -> Nullable<Varchar>,
    #[max_length = 500]
    address_line1 -> Nullable<Varchar>,
    #[max_length = 500]
    address_line2 -> Nullable<Varchar>,
    #[max_length = 255]
    city -> Nullable<Varchar>,
    #[max_length = 255]
    state -> Nullable<Varchar>,
    #[max_length = 50]
    postal_code -> Nullable<Varchar>,
    #[max_length = 100]
    country -> Nullable<Varchar>,
    description -> Nullable<Text>,
    tags -> Nullable<Array<Nullable<Text>>>,
    custom_fields -> Nullable<Jsonb>,
    owner_id -> Nullable<Uuid>,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::table! {
    crm_pipeline_stages (id) {

    id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 100]
    name -> Varchar,
    stage_order -> Int4,
    probability -> Int4,
    is_won -> Bool,
    is_lost -> Bool,
    #[max_length = 7]
    color -> Nullable<Varchar>,
    created_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::table! {
    crm_leads (id) {

    id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    contact_id -> Nullable<Uuid>,
    account_id -> Nullable<Uuid>,
    #[max_length = 500]
    title -> Varchar,
    description -> Nullable<Text>,
    value -> Nullable<Numeric>,
    #[max_length = 3]
    currency -> Nullable<Varchar>,
    stage_id -> Nullable<Uuid>,
    #[max_length = 100]
    stage -> Varchar,
    probability -> Int4,
    #[max_length = 100]
    source -> Nullable<Varchar>,
    expected_close_date -> Nullable<Date>,
    owner_id -> Nullable<Uuid>,
    #[max_length = 500]
    lost_reason -> Nullable<Varchar>,
    tags -> Nullable<Array<Nullable<Text>>>,
    custom_fields -> Nullable<Jsonb>,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    closed_at -> Nullable<Timestamptz>,
    branch_id -> Uuid,
    }
}


diesel::table! {
    crm_opportunities (id) {

    id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    lead_id -> Nullable<Uuid>,
    account_id -> Nullable<Uuid>,
    contact_id -> Nullable<Uuid>,
    #[max_length = 500]
    name -> Varchar,
    description -> Nullable<Text>,
    value -> Nullable<Numeric>,
    #[max_length = 3]
    currency -> Nullable<Varchar>,
    stage_id -> Nullable<Uuid>,
    #[max_length = 100]
    stage -> Varchar,
    probability -> Int4,
    #[max_length = 100]
    source -> Nullable<Varchar>,
    expected_close_date -> Nullable<Date>,
    actual_close_date -> Nullable<Date>,
    won -> Nullable<Bool>,
    owner_id -> Nullable<Uuid>,
    tags -> Nullable<Array<Nullable<Text>>>,
    custom_fields -> Nullable<Jsonb>,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::table! {
    crm_activities (id) {

    id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    contact_id -> Nullable<Uuid>,
    lead_id -> Nullable<Uuid>,
    opportunity_id -> Nullable<Uuid>,
    account_id -> Nullable<Uuid>,
    #[max_length = 50]
    activity_type -> Varchar,
    #[max_length = 500]
    subject -> Nullable<Varchar>,
    description -> Nullable<Text>,
    due_date -> Nullable<Timestamptz>,
    completed_at -> Nullable<Timestamptz>,
    #[max_length = 255]
    outcome -> Nullable<Varchar>,
    owner_id -> Nullable<Uuid>,
    created_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::table! {
    crm_notes (id) {

    id -> Uuid,
    org_id -> Uuid,
    contact_id -> Nullable<Uuid>,
    lead_id -> Nullable<Uuid>,
    opportunity_id -> Nullable<Uuid>,
    account_id -> Nullable<Uuid>,
    content -> Text,
    author_id -> Nullable<Uuid>,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    branch_id -> Nullable<Uuid>,
    }
}


// Moved verbatim from `botcontacts/src/schema.rs` (#1497) — column order
// preserved, so the consuming crate's `Queryable` structs are unaffected.
// Audit trail for CRM entity changes (migration `6.7.3-crm-audit-trail`).
diesel::table! {
    crm_audit_logs (id) {
        id -> Uuid,
        branch_id -> Uuid,
        entity -> Varchar,
        entity_id -> Nullable<Uuid>,
        action -> Varchar,
        actor_email -> Nullable<Varchar>,
        before -> Nullable<Jsonb>,
        after -> Nullable<Jsonb>,
        detail -> Nullable<Jsonb>,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(crm_notes -> branches (branch_id));
diesel::joinable!(crm_audit_logs -> branches (branch_id));

diesel::table! {
    // Physical order: `org_id`/`bot_id` first (tenant scope), `skills` is
    // `TEXT[]` (nullable elements).
    people (id) {
        id -> Uuid,
        org_id -> Uuid,
        bot_id -> Uuid,
        branch_id -> Uuid,
        user_id -> Nullable<Uuid>,
        first_name -> Varchar,
        last_name -> Nullable<Varchar>,
        email -> Nullable<Varchar>,
        phone -> Nullable<Varchar>,
        mobile -> Nullable<Varchar>,
        job_title -> Nullable<Varchar>,
        department -> Nullable<Varchar>,
        manager_id -> Nullable<Uuid>,
        office_location -> Nullable<Varchar>,
        hire_date -> Nullable<Date>,
        birthday -> Nullable<Date>,
        avatar_url -> Nullable<Text>,
        bio -> Nullable<Text>,
        skills -> Array<Nullable<Text>>,
        social_links -> Jsonb,
        custom_fields -> Jsonb,
        timezone -> Nullable<Varchar>,
        locale -> Nullable<Varchar>,
        is_active -> Bool,
        last_seen_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    people_teams (id) {

    id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 255]
    name -> Varchar,
    description -> Nullable<Text>,
    leader_id -> Nullable<Uuid>,
    parent_team_id -> Nullable<Uuid>,
    #[max_length = 20]
    color -> Nullable<Varchar>,
    #[max_length = 50]
    icon -> Nullable<Varchar>,
    is_active -> Bool,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::table! {
    people_team_members (id) {
        id -> Uuid,
        team_id -> Uuid,
        person_id -> Uuid,
        role -> Nullable<Varchar>,
        is_primary -> Bool,
        joined_at -> Timestamptz,
    }
}

diesel::table! {
    people_org_chart (id) {

    id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    person_id -> Uuid,
    reports_to_id -> Nullable<Uuid>,
    #[max_length = 255]
    position_title -> Nullable<Varchar>,
    position_level -> Int4,
    position_order -> Int4,
    effective_from -> Nullable<Date>,
    effective_until -> Nullable<Date>,
    is_active -> Bool,
    created_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::table! {
    people_departments (id) {

    id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 255]
    name -> Varchar,
    description -> Nullable<Text>,
    #[max_length = 50]
    code -> Nullable<Varchar>,
    parent_id -> Nullable<Uuid>,
    head_id -> Nullable<Uuid>,
    #[max_length = 50]
    cost_center -> Nullable<Varchar>,
    is_active -> Bool,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


mod part2;
pub use part2::*;
mod part3;
pub use part3::*;
