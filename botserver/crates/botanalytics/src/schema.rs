diesel::table! {
    bots (id) {
        id -> Uuid,
        branch_id -> Uuid,
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

// Canonical definition lives in `botschema` (#1497).
pub use botschema::organizations;
// Canonical definition lives in `botschema` (#1497).
pub use botschema::users;
diesel::table! {
    okr_objectives (id) {
        id -> Uuid,
        org_id -> Uuid,
        branch_id -> Uuid,
        owner_id -> Uuid,
        parent_id -> Nullable<Uuid>,
        title -> Varchar,
        description -> Nullable<Text>,
        period -> Varchar,
        period_start -> Nullable<Date>,
        period_end -> Nullable<Date>,
        status -> Varchar,
        progress -> Numeric,
        visibility -> Varchar,
        weight -> Numeric,
        tags -> Array<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

pub use botschema::okr_key_results;

pub use botschema::okr_checkins;

diesel::table! {
    okr_templates (id) {
        id -> Uuid,
        org_id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        description -> Nullable<Text>,
        category -> Nullable<Varchar>,
        objective_template -> Jsonb,
        key_result_templates -> Jsonb,
        is_system -> Bool,
        usage_count -> Integer,
        created_by -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    dashboards (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        slug -> Varchar,
        description -> Nullable<Text>,
        config -> Nullable<Jsonb>,
        is_default -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        owner_id -> Uuid,
        layout -> Jsonb,
        refresh_interval -> Nullable<Int4>,
        is_public -> Bool,
        is_template -> Bool,
        tags -> Text,
    }
}

// Canonical definition lives in `botschema` (#1497).
pub use botschema::dashboards::dashboard_widgets;

diesel::table! {
    dashboard_data_sources (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        source_type -> Varchar,
        config -> Nullable<Jsonb>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        description -> Nullable<Text>,
        connection -> Jsonb,
        schema_definition -> Jsonb,
        refresh_schedule -> Nullable<Varchar>,
        last_sync -> Nullable<Timestamptz>,
        status -> Varchar,
    }
}

// Canonical definition lives in `botschema` (#1497).
pub use botschema::dashboards::dashboard_filters;

// `organizations`, `users`, `dashboard_widgets` and `dashboard_filters` are
// re-exported from `botschema`; their join rules live there (#1497).
diesel::allow_tables_to_appear_in_same_query!(
    bots,
    okr_objectives,
    okr_templates,
    dashboards,
    dashboard_data_sources,
);
