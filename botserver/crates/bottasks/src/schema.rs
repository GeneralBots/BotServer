// @generated automatically by script from migration SQL for issue #707.
// Diesel table definitions for branch-scope cleanup.

diesel::table! {
    auto_tasks (id) {
        id -> Uuid,
        branch_id -> Uuid,
        bot_id -> Uuid,
        name -> Varchar,
        schedule -> Nullable<Varchar>,
        task_type -> Varchar,
        config -> Nullable<Jsonb>,
        is_active -> Nullable<Bool>,
        last_run_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    user_sessions (id) {
        id -> Uuid,
        branch_id -> Uuid,
        bot_id -> Uuid,
        session_id -> Varchar,
        user_id -> Nullable<Varchar>,
        data -> Nullable<Jsonb>,
        expires_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

// Canonical definition lives in `botschema` (#1497).
pub use botschema::tasks;


diesel::allow_tables_to_appear_in_same_query!(
    auto_tasks,
    user_sessions,
);
