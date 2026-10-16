use diesel::prelude::*;

table! {
    dashboards (id) {
        id -> Uuid,
        owner_id -> Uuid,
        name -> Text,
        description -> Nullable<Text>,
        layout -> Jsonb,
        refresh_interval -> Nullable<Int4>,
        is_public -> Bool,
        is_template -> Bool,
        tags -> Array<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    dashboard_widgets (id) {
        id -> Uuid,
        dashboard_id -> Uuid,
        widget_type -> Text,
        title -> Text,
        position_x -> Int4,
        position_y -> Int4,
        width -> Int4,
        height -> Int4,
        config -> Jsonb,
        data_query -> Nullable<Jsonb>,
        style -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    dashboard_data_sources (id) {
        id -> Uuid,
        name -> Text,
        description -> Nullable<Text>,
        source_type -> Text,
        connection -> Jsonb,
        schema_definition -> Jsonb,
        refresh_schedule -> Nullable<Text>,
        last_sync -> Nullable<Timestamptz>,
        status -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    dashboard_filters (id) {
        id -> Uuid,
        dashboard_id -> Uuid,
        name -> Text,
        field -> Text,
        filter_type -> Text,
        default_value -> Nullable<Jsonb>,
        options -> Jsonb,
        linked_widgets -> Jsonb,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    conversational_queries (id) {
        id -> Uuid,
        org_id -> Uuid,
        bot_id -> Uuid,
        dashboard_id -> Nullable<Uuid>,
        user_id -> Uuid,
        natural_language -> Text,
        generated_query -> Nullable<Text>,
        result_widget_config -> Nullable<Jsonb>,
        created_at -> Timestamptz,
        branch_id -> Uuid,
    }
}

joinable!(dashboard_widgets -> dashboards (dashboard_id));
joinable!(dashboard_filters -> dashboards (dashboard_id));
joinable!(conversational_queries -> dashboards (dashboard_id));

allow_tables_to_appear_in_same_query!(
    dashboards,
    dashboard_widgets,
    dashboard_data_sources,
    dashboard_filters,
    conversational_queries,
);
