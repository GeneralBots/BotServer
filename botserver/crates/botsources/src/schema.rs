// @generated automatically by script from migration SQL for issue #707.
// Diesel table definitions for branch-scope cleanup.

// Canonical definition lives in `botschema` (#1497).
pub use botschema::bot_configuration;

diesel::table! {
    connectors (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        connector_type -> Varchar,
        config -> Nullable<Jsonb>,
        is_active -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    delivery_transactions (id) {
        id -> Uuid,
        branch_id -> Uuid,
        transaction_id -> Varchar,
        amount -> Numeric,
        currency -> Nullable<Varchar>,
        status -> Nullable<Varchar>,
        source -> Nullable<Varchar>,
        destination -> Nullable<Varchar>,
        metadata -> Nullable<Jsonb>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    bank_transactions (id) {
        id -> Uuid,
        branch_id -> Uuid,
        external_id -> Nullable<Varchar>,
        description -> Nullable<Text>,
        amount -> Numeric,
        currency -> Nullable<Varchar>,
        transaction_date -> Date,
        category -> Nullable<Varchar>,
        reconciled -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    reconciliation_rules (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        rule_type -> Varchar,
        conditions -> Jsonb,
        action -> Varchar,
        is_active -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    reconciliation_runs (id) {
        id -> Uuid,
        branch_id -> Uuid,
        run_date -> Date,
        status -> Nullable<Varchar>,
        total_matched -> Nullable<Int4>,
        total_unmatched -> Nullable<Int4>,
        started_at -> Nullable<Timestamptz>,
        completed_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}


diesel::allow_tables_to_appear_in_same_query!(
    connectors,
    delivery_transactions,
    bank_transactions,
    reconciliation_rules,
    reconciliation_runs,
);
