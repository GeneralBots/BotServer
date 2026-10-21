//! External connector tables (canonical, #1497).

diesel::table! {
    connector_connections (id) {
        id -> Uuid,
        org_id -> Uuid,
        kind -> Text,
        display_name -> Nullable<Text>,
        vault_token_ref -> Text,
        status -> Text,
        cursors -> Jsonb,
        last_sync_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    indexed_items (id) {
        id -> Uuid,
        connection_id -> Uuid,
        external_id -> Text,
        title -> Text,
        body_tsv -> Nullable<Text>,
        vector_ref -> Nullable<Text>,
        acl -> Jsonb,
        container -> Nullable<Text>,
        external_url -> Nullable<Text>,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::joinable!(indexed_items -> connector_connections (connection_id));

diesel::allow_tables_to_appear_in_same_query!(connector_connections, indexed_items);

// Bank / delivery reconciliation tables (canonical, #1497).

diesel::table! {
    connectors (id) {

    id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 255]
    name -> Varchar,
    #[max_length = 50]
    connector_type -> Varchar,
    description -> Nullable<Text>,
    auth_config -> Jsonb,
    endpoints -> Jsonb,
    #[max_length = 100]
    schedule -> Nullable<Varchar>,
    is_active -> Bool,
    last_sync_at -> Nullable<Timestamptz>,
    #[max_length = 20]
    last_sync_status -> Nullable<Varchar>,
    error_log -> Nullable<Text>,
    created_at -> Timestamptz,
    updated_at -> Timestamptz,
    branch_id -> Uuid,
    #[max_length = 500]
    secrets_vault_path -> Nullable<Varchar>,
    last_test_at -> Nullable<Timestamptz>,
    #[max_length = 20]
    last_test_status -> Nullable<Varchar>,
    }
}

diesel::table! {
    delivery_transactions (id) {

    id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 50]
    platform -> Varchar,
    #[max_length = 255]
    platform_order_id -> Varchar,
    order_date -> Date,
    customer_name -> Nullable<Text>,
    items -> Nullable<Jsonb>,
    subtotal -> Numeric,
    delivery_fee -> Numeric,
    platform_commission -> Numeric,
    net_value -> Numeric,
    #[max_length = 50]
    payment_method -> Nullable<Varchar>,
    #[max_length = 50]
    status -> Varchar,
    reconciled -> Bool,
    reconciled_at -> Nullable<Timestamptz>,
    created_at -> Timestamptz,
    branch_id -> Uuid,
    }
}

diesel::table! {
    bank_transactions (id) {

    id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 100]
    bank -> Nullable<Varchar>,
    #[max_length = 50]
    account -> Nullable<Varchar>,
    transaction_date -> Date,
    description -> Text,
    amount -> Numeric,
    balance -> Nullable<Numeric>,
    #[max_length = 100]
    category -> Nullable<Varchar>,
    reconciled -> Bool,
    reconciled_at -> Nullable<Timestamptz>,
    matched_delivery_id -> Nullable<Uuid>,
    created_at -> Timestamptz,
    branch_id -> Uuid,
    }
}

diesel::table! {
    reconciliation_rules (id) {

    id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 255]
    name -> Varchar,
    #[max_length = 50]
    match_field -> Varchar,
    #[max_length = 20]
    match_operator -> Varchar,
    match_value -> Text,
    #[max_length = 100]
    category -> Nullable<Varchar>,
    auto_reconcile -> Bool,
    is_active -> Bool,
    created_at -> Timestamptz,
    branch_id -> Uuid,
    }
}

diesel::table! {
    reconciliation_runs (id) {

    id -> Uuid,
    bot_id -> Uuid,
    #[max_length = 50]
    status -> Varchar,
    matched_count -> Int4,
    unmatched_count -> Int4,
    total_amount_matched -> Numeric,
    started_at -> Timestamptz,
    completed_at -> Nullable<Timestamptz>,
    branch_id -> Uuid,
    }
}
