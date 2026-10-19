diesel::table! {
    crm_contacts (id) {
        id -> Uuid,
        branch_id -> Uuid,
        first_name -> Nullable<Varchar>,
        last_name -> Nullable<Varchar>,
        email -> Varchar,
        pass_hash -> Nullable<Varchar>,
        status -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

// `billing_subscriptions` used to be declared here; no migration ever created
// it and no code referenced it (the live subscription table is
// `billing_recurring`, below), so the declaration was deleted (#1517).

pub use botschema::billing_recurring;

pub use botschema::billing_payments;

pub use botschema::cloud_workspaces;

pub use botschema::workspace_resources;

pub use botschema::cloud_vouchers;

pub use botschema::bot_domains;

pub use botschema::cloud_voucher_redemptions;

pub use botschema::billing_customers;

pub use botschema::billing_payment_methods;

pub use botschema::cloud_audit_log;
