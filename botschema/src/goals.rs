use crate::core::{branches};

diesel::table! {
    okr_objectives (id) {
        id -> Uuid,
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
        tags -> Array<Nullable<Text>>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    okr_key_results (id) {
        id -> Uuid,
        org_id -> Uuid,
        branch_id -> Uuid,
        objective_id -> Uuid,
        owner_id -> Uuid,
        title -> Varchar,
        description -> Nullable<Text>,
        metric_type -> Varchar,
        start_value -> Numeric,
        target_value -> Numeric,
        current_value -> Numeric,
        unit -> Nullable<Varchar>,
        weight -> Numeric,
        status -> Varchar,
        due_date -> Nullable<Date>,
        scoring_type -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    okr_checkins (id) {
        id -> Uuid,
        org_id -> Uuid,
        branch_id -> Uuid,
        key_result_id -> Uuid,
        user_id -> Uuid,
        previous_value -> Nullable<Numeric>,
        new_value -> Numeric,
        note -> Nullable<Text>,
        confidence -> Nullable<Varchar>,
        blockers -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    okr_alignments (id) {
        id -> Uuid,
        branch_id -> Uuid,
        child_objective_id -> Uuid,
        parent_objective_id -> Uuid,
        alignment_type -> Varchar,
        weight -> Numeric,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    okr_templates (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        description -> Nullable<Text>,
        category -> Nullable<Varchar>,
        objective_template -> Jsonb,
        key_result_templates -> Jsonb,
        is_system -> Bool,
        usage_count -> Int4,
        created_by -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    okr_comments (id) {
        id -> Uuid,
        branch_id -> Uuid,
        objective_id -> Nullable<Uuid>,
        key_result_id -> Nullable<Uuid>,
        user_id -> Uuid,
        content -> Text,
        parent_comment_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    okr_activity_log (id) {
        id -> Uuid,
        branch_id -> Uuid,
        objective_id -> Nullable<Uuid>,
        key_result_id -> Nullable<Uuid>,
        user_id -> Uuid,
        activity_type -> Varchar,
        description -> Nullable<Text>,
        old_value -> Nullable<Text>,
        new_value -> Nullable<Text>,
        metadata -> Jsonb,
        created_at -> Timestamptz,
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
