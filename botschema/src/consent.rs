//! App permission / consent audit tables (canonical, #1497).

diesel::table! {
    app_permissions (id) {
        id -> Uuid,
        user_id -> Uuid,
        org_id -> Nullable<Uuid>,
        branch_id -> Nullable<Uuid>,
        app_id -> Varchar,
        action_class -> Varchar,
        scope -> Jsonb,
        granted -> Bool,
        granted_via -> Varchar,
        expires_at -> Nullable<Timestamptz>,
        granted_at -> Timestamptz,
    }
}

diesel::table! {
    consent_audit (id) {
        id -> Uuid,
        permission_id -> Nullable<Uuid>,
        user_id -> Nullable<Uuid>,
        request -> Jsonb,
        outcome -> Varchar,
        decided_by -> Nullable<Uuid>,
        created_at -> Timestamptz,
    }
}

diesel::allow_tables_to_appear_in_same_query!(app_permissions, consent_audit);
