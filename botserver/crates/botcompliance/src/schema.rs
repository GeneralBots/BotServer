// @generated from the actual migrations (6.0.21-01-compliance + 9.23-branch-scope-cleanup).
// Do not edit by hand; keep in sync with migrations/*/up.sql.

diesel::table! {
    compliance_checks (id) {
        id -> Uuid,
        org_id -> Nullable<Uuid>,
        bot_id -> Nullable<Uuid>,
        check_type -> Varchar,
        target_type -> Nullable<Varchar>,
        status -> Nullable<Varchar>,
        checked_at -> Nullable<Timestamptz>,
        checked_by -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        branch_id -> Uuid,
        target_id -> Nullable<Uuid>,
        result -> Nullable<Jsonb>,
    }
}

pub use botschema::compliance_issues;

pub use botschema::compliance_audit_log;

pub use botschema::compliance_evidence;

pub use botschema::compliance_risk_assessments;

pub use botschema::compliance_training_records;

pub use botschema::compliance_access_reviews;

diesel::table! {
    compliance_frameworks (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        version -> Varchar,
        description -> Nullable<Text>,
        framework_key -> Varchar,
        status -> Varchar,
        created_by -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    compliance_controls (id) {
        id -> Uuid,
        branch_id -> Uuid,
        framework_id -> Uuid,
        control_id -> Varchar,
        title -> Varchar,
        description -> Nullable<Text>,
        category -> Nullable<Varchar>,
        is_mandatory -> Bool,
        version -> Varchar,
        status -> Varchar,
        created_by -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    compliance_control_evidence (id) {
        id -> Uuid,
        branch_id -> Uuid,
        control_id -> Uuid,
        file_path -> Varchar,
        description -> Nullable<Text>,
        evidence_type -> Varchar,
        status -> Varchar,
        owner_id -> Nullable<Uuid>,
        approved_by -> Nullable<Uuid>,
        approved_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    compliance_checks,
    compliance_frameworks,
    compliance_controls,
    compliance_control_evidence,
);
