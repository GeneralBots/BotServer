// use crate::core::{bots, branches, organizations};

diesel::table! {
    projects (id) {
        id -> Uuid,
        name -> Varchar,
        description -> Nullable<Text>,
        status -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        org -> Varchar,
        project_type -> Varchar,
        deploy_target -> Varchar,
        repo_url -> Nullable<Text>,
        deploy_url -> Nullable<Text>,
        container_name -> Nullable<Varchar>,
        custom_domain -> Nullable<Varchar>,
        environment -> Varchar,
        framework -> Nullable<Varchar>,
    }
}

diesel::table! {
    project_tasks (id) {
        id -> Uuid,
        project_id -> Uuid,
        parent_id -> Nullable<Uuid>,
        name -> Text,
        description -> Nullable<Text>,
        task_type -> Text,
        start_date -> Date,
        end_date -> Date,
        duration_days -> Int4,
        percent_complete -> Int4,
        status -> Text,
        priority -> Text,
        assigned_to -> Array<Nullable<Uuid>>,
        estimated_hours -> Nullable<Float4>,
        actual_hours -> Nullable<Float4>,
        cost -> Nullable<Float8>,
        notes -> Nullable<Text>,
        wbs -> Text,
        outline_level -> Int4,
        is_milestone -> Bool,
        is_summary -> Bool,
        is_critical -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    project_resources (id) {
        id -> Uuid,
        project_id -> Uuid,
        user_id -> Nullable<Uuid>,
        name -> Text,
        resource_type -> Text,
        email -> Nullable<Text>,
        max_units -> Float4,
        standard_rate -> Nullable<Float8>,
        overtime_rate -> Nullable<Float8>,
        cost_per_use -> Nullable<Float8>,
        calendar_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    project_assignments (id) {
        id -> Uuid,
        task_id -> Uuid,
        resource_id -> Uuid,
        units -> Float4,
        work_hours -> Float4,
        start_date -> Date,
        end_date -> Date,
        cost -> Float8,
    }
}

diesel::table! {
    project_dependencies (id) {
        id -> Uuid,
        task_id -> Uuid,
        predecessor_id -> Uuid,
        dependency_type -> Text,
        lag_days -> Int4,
    }
}

diesel::joinable!(project_tasks -> projects (project_id));
diesel::joinable!(project_resources -> projects (project_id));
diesel::joinable!(project_assignments -> project_tasks (task_id));
diesel::joinable!(project_assignments -> project_resources (resource_id));
diesel::joinable!(project_dependencies -> project_tasks (task_id));

diesel::allow_tables_to_appear_in_same_query!(
    projects,
    project_tasks,
    project_resources,
    project_assignments,
    project_dependencies,
);
