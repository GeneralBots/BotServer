// Diesel table definitions for dashboards (migrations 6.0.19 + 6.5.23).

diesel::table! {
    dashboards (id) {
        id -> Uuid,
        org_id -> Uuid,
        bot_id -> Uuid,
        owner_id -> Uuid,
        name -> Varchar,
        description -> Nullable<Text>,
        layout -> Jsonb,
        refresh_interval -> Nullable<Int4>,
        is_public -> Bool,
        is_template -> Bool,
        tags -> Array<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        branch_id -> Uuid,
    }
}

diesel::table! {
    dashboard_data_sources (id) {
        id -> Uuid,
        org_id -> Uuid,
        bot_id -> Uuid,
        name -> Varchar,
        description -> Nullable<Text>,
        source_type -> Varchar,
        connection -> Jsonb,
        schema_definition -> Jsonb,
        refresh_schedule -> Nullable<Varchar>,
        last_sync -> Nullable<Timestamptz>,
        status -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        branch_id -> Uuid,
    }
}

// Canonical definition lives in `botschema::dashboards` (#1497).
pub use botschema::dashboards::dashboard_widgets;

// Canonical definition lives in `botschema::dashboards` (#1497).
pub use botschema::dashboards::dashboard_filters;

// `botschema` does not re-export the `dashboards` module at the root (its own
// `dashboards` table would collide with the module name), so the canonical path
// goes through the module.
pub use botschema::dashboards::conversational_queries;

// `dashboard_widgets` and `dashboard_filters` are re-exported from
// `botschema::dashboards`; their join rules live there (#1497).
diesel::allow_tables_to_appear_in_same_query!(
    dashboards,
    dashboard_data_sources,
);
