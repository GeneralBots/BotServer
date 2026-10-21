use diesel::prelude::*;

diesel::table! {
    dashboards (id) {

    id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    owner_id -> Uuid,
    #[max_length = 255]
    name -> Varchar,
    description -> Nullable<Text>,
    layout -> Jsonb,
    refresh_interval -> Nullable<Int4>,
    is_public -> Bool,
    is_template -> Bool,
    tags -> Array<Nullable<Text>>,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


table! {
    dashboard_widgets (id) {
        id -> Uuid,
        dashboard_id -> Uuid,
        #[max_length = 50]
        widget_type -> Varchar,
        #[max_length = 255]
        title -> Varchar,
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

diesel::table! {
    dashboard_data_sources (id) {
        id -> Uuid,
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
        org_id -> Uuid,
        bot_id -> Uuid,
        branch_id -> Uuid,
    }
}

table! {
    dashboard_filters (id) {
        id -> Uuid,
        dashboard_id -> Uuid,
        #[max_length = 255]
        name -> Varchar,
        #[max_length = 255]
        field -> Varchar,
        #[max_length = 50]
        filter_type -> Varchar,
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
