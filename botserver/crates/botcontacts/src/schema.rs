// Canonical definition lives in `botschema` (#1497).
pub use botschema::bots;

pub use botschema::organizations;

diesel::table! {
    crm_contacts (id) {
        id -> Uuid,
        org_id -> Uuid,
        bot_id -> Uuid,
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
        tags -> Nullable<Array<Text>>,
        custom_fields -> Nullable<Jsonb>,
        notes -> Nullable<Text>,
        owner_id -> Nullable<Uuid>,
        pass_hash -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        address_line1 -> Nullable<Varchar>,
        address_line2 -> Nullable<Varchar>,
        city -> Nullable<Varchar>,
        state -> Nullable<Varchar>,
        postal_code -> Nullable<Varchar>,
        country -> Nullable<Varchar>,
    }
}

// Canonical definition lives in `botschema` (#1497).
pub use botschema::crm_accounts;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::crm_pipeline_stages;

pub use botschema::crm_leads;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::crm_opportunities;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::crm_activities;

pub use botschema::crm_notes;

// #1441 P2 — append-only audit trail for destructive/state-changing CRM
// operations (migration 6.7.3-crm-audit-trail).
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

// Canonical definition lives in `botschema` (#1497).
pub use botschema::crm_deals;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::marketing_campaigns;

// Canonical definitions live in `botschema` (#1497): `calendars` there is the
// 14-column shape (the local copy carried a ghost `is_public` and a nullable
// `owner_id`), `calendar_events` the 24-column one (the local copy carried ghost
// `starts_at`/`ends_at`/`is_all_day` and typed `attendees` as text).
pub use botschema::{calendar_events, calendars};

pub use botschema::calendar_event_attendees;

pub use botschema::calendar_event_reminders;

pub use botschema::calendar_shares;

diesel::table! {
    people (id) {
        id -> Uuid,
        branch_id -> Uuid,
        first_name -> Varchar,
        last_name -> Nullable<Varchar>,
        email -> Nullable<Varchar>,
        phone -> Nullable<Varchar>,
        job_title -> Nullable<Varchar>,
        department_id -> Nullable<Uuid>,
        manager_id -> Nullable<Uuid>,
        status -> Nullable<Varchar>,
        hire_date -> Nullable<Date>,
        metadata -> Nullable<Jsonb>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        user_id -> Nullable<Uuid>,
        mobile -> Nullable<Varchar>,
        department -> Nullable<Varchar>,
        office_location -> Nullable<Varchar>,
        birthday -> Nullable<Date>,
        avatar_url -> Nullable<Text>,
        bio -> Nullable<Text>,
        skills -> Text,
        social_links -> Jsonb,
        custom_fields -> Jsonb,
        timezone -> Nullable<Varchar>,
        locale -> Nullable<Varchar>,
        is_active -> Bool,
        last_seen_at -> Nullable<Timestamptz>,
    }
}

// Canonical definition lives in `botschema` (#1497).
pub use botschema::people_departments;

diesel::table! {
    tasks (id) {
        id -> Uuid,
        branch_id -> Uuid,
        title -> Varchar,
        description -> Nullable<Text>,
        status -> Nullable<Varchar>,
        priority -> Nullable<Int4>,
        assignee_id -> Nullable<Uuid>,
        due_date -> Nullable<Timestamptz>,
        completed_at -> Nullable<Timestamptz>,
        parent_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        reporter_id -> Nullable<Uuid>,
        project_id -> Nullable<Uuid>,
        tags -> Text,
        dependencies -> Text,
        estimated_hours -> Nullable<Float8>,
        actual_hours -> Nullable<Float8>,
        progress -> Int4,
    }
}

