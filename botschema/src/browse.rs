//! Browser task/policy tables (canonical, #1497).

diesel::table! {
    browser_tasks (id) {
        id -> Uuid,
        user_id -> Uuid,
        org_id -> Nullable<Uuid>,
        bot_id -> Nullable<Uuid>,
        goal -> Text,
        domains -> Jsonb,
        budget_steps -> Int4,
        status -> Varchar,
        plan -> Nullable<Jsonb>,
        progress -> Nullable<Jsonb>,
        result -> Nullable<Jsonb>,
        citations -> Nullable<Jsonb>,
        error -> Nullable<Text>,
        started_at -> Nullable<Timestamptz>,
        finished_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    page_facts (id) {
        id -> Uuid,
        user_id -> Uuid,
        url -> Text,
        title -> Nullable<Text>,
        facts -> Jsonb,
        visit_count -> Int4,
        last_seen -> Timestamptz,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    browse_sessions (id) {
        id -> Uuid,
        user_id -> Uuid,
        task_id -> Nullable<Uuid>,
        started_at -> Timestamptz,
        ended_at -> Nullable<Timestamptz>,
        summary -> Nullable<Text>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(browser_tasks, page_facts, browse_sessions);
