// SaaS / cloud control-plane tables.
//
// Canonical home (#1497). These tables were declared only inside
// `botcloud/src/schema_ext.rs`; the declarations were moved here verbatim, so
// the column order (and therefore every `#[derive(Queryable)]` mapping in the
// consuming crate) is unchanged. Verified against the database by
// `scripts/schema_db_audit.py` — see `issues/schema/db_audit.md` (#1519).

diesel::table! {
    cloud_workspaces (id) {
        id -> Uuid,
        org_id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        description -> Nullable<Text>,
        icon -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    workspace_resources (id) {
        id -> Uuid,
        workspace_id -> Uuid,
        org_id -> Uuid,
        store_item_id -> Varchar,
        name -> Varchar,
        resource_type -> Varchar,
        status -> Varchar,
        config -> Nullable<Jsonb>,
        provisioned_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    cloud_vouchers (id) {
        id -> Uuid,
        code -> Varchar,
        plan -> Varchar,
        trial_days -> Int4,
        max_uses -> Int4,
        uses_count -> Int4,
        created_by -> Nullable<Uuid>,
        expires_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    cloud_voucher_redemptions (id) {
        id -> Uuid,
        voucher_id -> Uuid,
        contact_id -> Uuid,
        org_id -> Uuid,
        branch_id -> Uuid,
        subscription_id -> Nullable<Uuid>,
        trial_days -> Int4,
        redeemed_at -> Timestamptz,
    }
}

diesel::table! {
    cloud_audit_log (id) {
        id -> Uuid,
        branch_id -> Uuid,
        actor_email -> Nullable<Varchar>,
        action -> Varchar,
        entity -> Varchar,
        entity_id -> Nullable<Varchar>,
        details -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    billing_customers (id) {
        id -> Uuid,
        branch_id -> Uuid,
        stripe_customer_id -> Varchar,
        email -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    billing_payment_methods (id) {
        id -> Uuid,
        branch_id -> Uuid,
        stripe_customer_id -> Varchar,
        stripe_pm_id -> Varchar,
        brand -> Varchar,
        last4 -> Varchar,
        exp_month -> Int4,
        exp_year -> Int4,
        is_default -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    bot_domains (id) {
        id -> Uuid,
        domain -> Varchar,
        bot_id -> Uuid,
        org_id -> Nullable<Uuid>,
        branch_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}