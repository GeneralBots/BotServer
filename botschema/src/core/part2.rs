//! Split from `core.rs` — 241 lines.

use super::*;

diesel::table! {
    rbac_roles (id) {
        id -> Uuid,
        name -> Varchar,
        display_name -> Varchar,
        description -> Nullable<Text>,
        is_system -> Bool,
        is_active -> Bool,
        created_by -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    rbac_groups (id) {
        id -> Uuid,
        name -> Varchar,
        display_name -> Varchar,
        description -> Nullable<Text>,
        parent_group_id -> Nullable<Uuid>,
        is_active -> Bool,
        created_by -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    rbac_permissions (id) {
        id -> Uuid,
        name -> Varchar,
        display_name -> Varchar,
        description -> Nullable<Text>,
        resource_type -> Varchar,
        action -> Varchar,
        category -> Varchar,
        is_system -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    rbac_role_permissions (id) {
        id -> Uuid,
        role_id -> Uuid,
        permission_id -> Uuid,
        granted_by -> Nullable<Uuid>,
        granted_at -> Timestamptz,
    }
}

diesel::table! {
    rbac_user_roles (id) {
        id -> Uuid,
        user_id -> Uuid,
        role_id -> Uuid,
        granted_by -> Nullable<Uuid>,
        granted_at -> Timestamptz,
        expires_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    rbac_user_groups (id) {
        id -> Uuid,
        user_id -> Uuid,
        group_id -> Uuid,
        added_by -> Nullable<Uuid>,
        added_at -> Timestamptz,
    }
}

diesel::table! {
    rbac_group_roles (id) {
        id -> Uuid,
        group_id -> Uuid,
        role_id -> Uuid,
        granted_by -> Nullable<Uuid>,
        granted_at -> Timestamptz,
    }
}

diesel::joinable!(rbac_role_permissions -> rbac_roles (role_id));
diesel::joinable!(rbac_role_permissions -> rbac_permissions (permission_id));
diesel::joinable!(rbac_user_roles -> users (user_id));
diesel::joinable!(rbac_user_roles -> rbac_roles (role_id));
diesel::joinable!(rbac_user_groups -> users (user_id));
diesel::joinable!(rbac_user_groups -> rbac_groups (group_id));
diesel::joinable!(rbac_group_roles -> rbac_groups (group_id));
diesel::joinable!(rbac_group_roles -> rbac_roles (role_id));
diesel::joinable!(website_crawls -> bots (bot_id));
diesel::joinable!(organization_invitations -> organizations (org_id));
diesel::joinable!(organization_invitations -> branches (branch_id));

diesel::table! {
    user_organizations (id) {

    id -> Uuid,
    user_id -> Uuid,
    org_id -> Uuid,
    #[max_length = 50]
    role -> Nullable<Varchar>,
    is_default -> Nullable<Bool>,
    joined_at -> Timestamptz,
    }
}


diesel::joinable!(user_organizations -> users (user_id));
diesel::joinable!(user_organizations -> organizations (org_id));

diesel::table! {
    user_preferences (id) {
        id -> Uuid,
        user_id -> Uuid,
        preference_key -> Varchar,
        preference_value -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    user_login_tokens (id) {
        id -> Uuid,
        user_id -> Uuid,
        token_hash -> Varchar,
        expires_at -> Timestamptz,
        created_at -> Timestamptz,
        last_used -> Timestamptz,
        user_agent -> Nullable<Text>,
        ip_address -> Nullable<Varchar>,
        is_active -> Bool,
    }
}

diesel::table! {
    website_crawls (id) {

    id -> Uuid,
    bot_id -> Uuid,
    url -> Text,
    last_crawled -> Nullable<Timestamptz>,
    next_crawl -> Nullable<Timestamptz>,
    #[max_length = 20]
    expires_policy -> Varchar,
    max_depth -> Nullable<Int4>,
    max_pages -> Nullable<Int4>,
    crawl_status -> Nullable<Int2>,
    pages_crawled -> Nullable<Int4>,
    error_message -> Nullable<Text>,
    created_at -> Nullable<Timestamptz>,
    updated_at -> Nullable<Timestamptz>,
    #[max_length = 20]
    refresh_policy -> Nullable<Varchar>,
    branch_id -> Uuid,
    }
}


diesel::table! {
    // Physical order from `6.0.0-01-core`; `branch_id` appended by
    // `6.5.23-branch-scope-cleanup` and `stage` by a later ALTER. `status`,
    // `priority` and `stage` are TEXT (not Int/Varchar); `tags` and
    // `dependencies` are nullable arrays; there is no `parent_id` column.
    tasks (id) {
        id -> Uuid,
        title -> Text,
        description -> Nullable<Text>,
        status -> Text,
        priority -> Text,
        assignee_id -> Nullable<Uuid>,
        reporter_id -> Nullable<Uuid>,
        project_id -> Nullable<Uuid>,
        due_date -> Nullable<Timestamptz>,
        tags -> Nullable<Array<Nullable<Text>>>,
        dependencies -> Nullable<Array<Nullable<Uuid>>>,
        estimated_hours -> Nullable<Float8>,
        actual_hours -> Nullable<Float8>,
        progress -> Nullable<Int4>,
        created_at -> Nullable<Timestamptz>,
        updated_at -> Nullable<Timestamptz>,
        completed_at -> Nullable<Timestamptz>,
        branch_id -> Uuid,
        stage -> Text,
        // Subtask hierarchy (issue #878); added by migration 6.7.11-tasks-parent-id.
        parent_id -> Nullable<Uuid>,
    }
}

diesel::table! {
    kb_collections (id) {
        id -> Uuid,
        bot_id -> Uuid,
        name -> Text,
        folder_path -> Text,
        qdrant_collection -> Text,
        document_count -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    kb_group_associations (id) {

    id -> Uuid,
    kb_id -> Uuid,
    group_id -> Uuid,
    granted_by -> Nullable<Uuid>,
    granted_at -> Timestamptz,
    }
}


diesel::joinable!(kb_collections -> bots (bot_id));
diesel::joinable!(kb_group_associations -> kb_collections (kb_id));

diesel::allow_tables_to_appear_in_same_query!(
    rbac_roles,
    rbac_groups,
    rbac_permissions,
    rbac_role_permissions,
    rbac_user_roles,
    rbac_user_groups,
    rbac_group_roles,
    users,
    user_organizations,
    website_crawls,
    bots,
    bot_configuration,
    organizations,
    organization_invitations,
    tasks,
    kb_collections,
    kb_group_associations,
    branches,
    user_sessions,
    bot_memories,
    basic_tools,
    system_automations,
    workflow_executions,
    workflow_events,
    tenants,
);

/// Resolve the branch that owns `bot` for a branch-scoped `system_automations` row.
///
/// Migration `6.5.23-branch-scope-cleanup` made `system_automations.branch_id`
/// `NOT NULL` with a foreign key to `branches`, so every trigger registration
/// must supply it. Registrations that omit it are rejected by PostgreSQL with a
/// not-null violation, which is why `ON UPDATE OF`/`WEBHOOK`/`SET SCHEDULE`
/// compilation used to fail. Returns the error instead of guessing a branch so
/// callers can report which bot could not be resolved.
pub fn system_automation_branch_id(
    conn: &mut diesel::PgConnection,
    bot: uuid::Uuid,
) -> Result<uuid::Uuid, diesel::result::Error> {
    use diesel::prelude::*;

    bots::table
        .filter(bots::id.eq(bot))
        .select(bots::branch_id)
        .first::<uuid::Uuid>(conn)
}

// AutoTask runtime table (canonical, #1497).

diesel::table! {
    auto_tasks (id) {

    id -> Uuid,
    bot_id -> Uuid,
    session_id -> Nullable<Uuid>,
    #[max_length = 500]
    title -> Varchar,
    intent -> Text,
    #[max_length = 50]
    status -> Varchar,
    #[max_length = 50]
    execution_mode -> Varchar,
    #[max_length = 20]
    priority -> Varchar,
    plan_id -> Nullable<Uuid>,
    basic_program -> Nullable<Text>,
    current_step -> Nullable<Int4>,
    total_steps -> Nullable<Int4>,
    progress -> Nullable<Float8>,
    step_results -> Nullable<Jsonb>,
    manifest_json -> Nullable<Jsonb>,
    error -> Nullable<Text>,
    started_at -> Nullable<Timestamptz>,
    completed_at -> Nullable<Timestamptz>,
    created_at -> Nullable<Timestamptz>,
    updated_at -> Nullable<Timestamptz>,
    branch_id -> Uuid,
    #[max_length = 500]
    name -> Varchar,
    #[max_length = 255]
    schedule -> Nullable<Varchar>,
    #[max_length = 100]
    task_type -> Varchar,
    config -> Nullable<Jsonb>,
    is_active -> Bool,
    last_run_at -> Nullable<Timestamptz>,
    }
}
