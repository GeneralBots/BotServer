use crate::core::{branches};

diesel::table! {
    okr_objectives (id) {

    id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    owner_id -> Uuid,
    parent_id -> Nullable<Uuid>,
    #[max_length = 500]
    title -> Varchar,
    description -> Nullable<Text>,
    #[max_length = 50]
    period -> Varchar,
    period_start -> Nullable<Date>,
    period_end -> Nullable<Date>,
    #[max_length = 50]
    status -> Varchar,
    progress -> Numeric,
    #[max_length = 50]
    visibility -> Varchar,
    weight -> Numeric,
    tags -> Array<Nullable<Text>>,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::table! {
    okr_key_results (id) {

    id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    objective_id -> Uuid,
    owner_id -> Uuid,
    #[max_length = 500]
    title -> Varchar,
    description -> Nullable<Text>,
    #[max_length = 50]
    metric_type -> Varchar,
    start_value -> Numeric,
    target_value -> Numeric,
    current_value -> Numeric,
    #[max_length = 50]
    unit -> Nullable<Varchar>,
    weight -> Numeric,
    #[max_length = 50]
    status -> Varchar,
    due_date -> Nullable<Date>,
    #[max_length = 50]
    scoring_type -> Varchar,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::table! {
    okr_checkins (id) {

    id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    key_result_id -> Uuid,
    user_id -> Uuid,
    previous_value -> Nullable<Numeric>,
    new_value -> Numeric,
    note -> Nullable<Text>,
    #[max_length = 50]
    confidence -> Nullable<Varchar>,
    blockers -> Nullable<Text>,
    created_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::table! {
    okr_alignments (id) {

    id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    child_objective_id -> Uuid,
    parent_objective_id -> Uuid,
    #[max_length = 50]
    alignment_type -> Varchar,
    weight -> Numeric,
    created_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::table! {
    okr_templates (id) {

    id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 255]
    name -> Varchar,
    description -> Nullable<Text>,
    #[max_length = 100]
    category -> Nullable<Varchar>,
    objective_template -> Jsonb,
    key_result_templates -> Jsonb,
    is_system -> Bool,
    usage_count -> Int4,
    created_by -> Nullable<Uuid>,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::table! {
    okr_comments (id) {

    id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    objective_id -> Nullable<Uuid>,
    key_result_id -> Nullable<Uuid>,
    user_id -> Uuid,
    content -> Text,
    parent_comment_id -> Nullable<Uuid>,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::table! {
    okr_activity_log (id) {

    id -> Uuid,
    org_id -> Uuid,
    bot_id -> Uuid,
    objective_id -> Nullable<Uuid>,
    key_result_id -> Nullable<Uuid>,
    user_id -> Uuid,
    #[max_length = 50]
    activity_type -> Varchar,
    description -> Nullable<Text>,
    old_value -> Nullable<Text>,
    new_value -> Nullable<Text>,
    metadata -> Jsonb,
    created_at -> Timestamptz,
    branch_id -> Uuid,
    }
}


diesel::joinable!(okr_objectives -> branches (branch_id));
diesel::joinable!(okr_key_results -> branches (branch_id));
diesel::joinable!(okr_key_results -> okr_objectives (objective_id));
diesel::joinable!(okr_checkins -> branches (branch_id));
diesel::joinable!(okr_checkins -> okr_key_results (key_result_id));
diesel::joinable!(okr_alignments -> branches (branch_id));
diesel::joinable!(okr_templates -> branches (branch_id));
diesel::joinable!(okr_comments -> branches (branch_id));
diesel::joinable!(okr_activity_log -> branches (branch_id));
