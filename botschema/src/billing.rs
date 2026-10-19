use crate::core::{branches};

diesel::table! {
    billing_invoices (id) {
        id -> Uuid,
        branch_id -> Uuid,
        invoice_number -> Varchar,
        customer_id -> Nullable<Uuid>,
        customer_name -> Nullable<Varchar>,
        customer_email -> Nullable<Varchar>,
        customer_address -> Nullable<Text>,
        status -> Nullable<Varchar>,
        issue_date -> Date,
        due_date -> Nullable<Date>,
        subtotal -> Numeric,
        tax_rate -> Numeric,
        tax_amount -> Numeric,
        discount_percent -> Numeric,
        discount_amount -> Numeric,
        total -> Nullable<Numeric>,
        amount_paid -> Numeric,
        amount_due -> Numeric,
        currency -> Nullable<Varchar>,
        notes -> Nullable<Text>,
        terms -> Nullable<Text>,
        footer -> Nullable<Text>,
        paid_at -> Nullable<Timestamptz>,
        sent_at -> Nullable<Timestamptz>,
        voided_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        org_id -> Uuid,
        bot_id -> Uuid,
    }
}

diesel::table! {
    billing_invoice_items (id) {
        id -> Uuid,
        invoice_id -> Uuid,
        product_id -> Nullable<Uuid>,
        description -> Varchar,
        quantity -> Numeric,
        unit_price -> Numeric,
        discount_percent -> Numeric,
        tax_rate -> Numeric,
        amount -> Numeric,
        sort_order -> Int4,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    billing_payments (id) {
        id -> Uuid,
        branch_id -> Uuid,
        invoice_id -> Nullable<Uuid>,
        payment_number -> Varchar,
        amount -> Numeric,
        currency -> Nullable<Varchar>,
        payment_method -> Nullable<Varchar>,
        payment_reference -> Nullable<Varchar>,
        status -> Nullable<Varchar>,
        payer_name -> Nullable<Varchar>,
        payer_email -> Nullable<Varchar>,
        notes -> Nullable<Text>,
        paid_at -> Nullable<Timestamptz>,
        refunded_at -> Nullable<Timestamptz>,
        refund_amount -> Nullable<Numeric>,
        created_at -> Timestamptz,
        org_id -> Uuid,
        bot_id -> Uuid,
    }
}

diesel::table! {
    billing_quotes (id) {
        id -> Uuid,
        branch_id -> Uuid,
        quote_number -> Varchar,
        customer_id -> Nullable<Uuid>,
        customer_name -> Nullable<Varchar>,
        customer_email -> Nullable<Varchar>,
        customer_address -> Nullable<Text>,
        status -> Nullable<Varchar>,
        issue_date -> Date,
        valid_until -> Nullable<Date>,
        subtotal -> Numeric,
        tax_rate -> Numeric,
        tax_amount -> Numeric,
        discount_percent -> Numeric,
        discount_amount -> Numeric,
        total -> Nullable<Numeric>,
        currency -> Nullable<Varchar>,
        notes -> Nullable<Text>,
        terms -> Nullable<Text>,
        accepted_at -> Nullable<Timestamptz>,
        rejected_at -> Nullable<Timestamptz>,
        converted_invoice_id -> Nullable<Uuid>,
        sent_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        org_id -> Uuid,
        bot_id -> Uuid,
    }
}

diesel::table! {
    billing_quote_items (id) {
        id -> Uuid,
        quote_id -> Uuid,
        product_id -> Nullable<Uuid>,
        description -> Varchar,
        quantity -> Numeric,
        unit_price -> Numeric,
        discount_percent -> Numeric,
        tax_rate -> Numeric,
        amount -> Numeric,
        sort_order -> Int4,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    billing_recurring (id) {
        id -> Uuid,
        branch_id -> Uuid,
        customer_id -> Nullable<Uuid>,
        customer_name -> Varchar,
        customer_email -> Nullable<Varchar>,
        status -> Nullable<Varchar>,
        frequency -> Varchar,
        interval_count -> Int4,
        amount -> Nullable<Numeric>,
        currency -> Nullable<Varchar>,
        description -> Nullable<Text>,
        next_invoice_date -> Date,
        last_invoice_date -> Nullable<Date>,
        last_invoice_id -> Nullable<Uuid>,
        start_date -> Date,
        end_date -> Nullable<Date>,
        invoices_generated -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        org_id -> Uuid,
        bot_id -> Uuid,
    }
}

diesel::table! {
    billing_tax_rates (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        rate -> Numeric,
        description -> Nullable<Text>,
        region -> Nullable<Varchar>,
        is_default -> Nullable<Bool>,
        is_active -> Bool,
        created_at -> Timestamptz,
        org_id -> Uuid,
        bot_id -> Uuid,
    }
}

diesel::table! {
    products (id) {
        id -> Uuid,
        branch_id -> Uuid,
        sku -> Nullable<Varchar>,
        name -> Varchar,
        description -> Nullable<Text>,
        category -> Nullable<Varchar>,
        product_type -> Varchar,
        price -> Numeric,
        cost -> Nullable<Numeric>,
        currency -> Varchar,
        tax_rate -> Numeric,
        unit -> Varchar,
        stock_quantity -> Int4,
        low_stock_threshold -> Int4,
        is_active -> Bool,
        images -> Jsonb,
        attributes -> Jsonb,
        weight -> Nullable<Numeric>,
        dimensions -> Nullable<Jsonb>,
        barcode -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

// `services` is created by `6.0.10-01-products`; `6.5.23-branch-scope-cleanup`
// adds `branch_id` (NOT NULL) and `6.5.40-services-tax` adds `tax_rate`.
diesel::table! {
    services (id) {
        id -> Uuid,
        org_id -> Uuid,
        bot_id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        description -> Nullable<Text>,
        category -> Nullable<Varchar>,
        service_type -> Varchar,
        hourly_rate -> Nullable<Numeric>,
        fixed_price -> Nullable<Numeric>,
        currency -> Varchar,
        duration_minutes -> Nullable<Int4>,
        is_active -> Bool,
        attributes -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        tax_rate -> Numeric,
    }
}

// `product_categories` is created by `6.0.10-01-products`; `6.5.15.1-consolidated`
// and `6.5.98-product-categories-display-order` add `display_order` + `updated_at`
// and make `org_id`/`bot_id` nullable (the botproducts seed omits them), while
// `6.5.23-branch-scope-cleanup` adds `branch_id` as NOT NULL.
diesel::table! {
    product_categories (id) {
        id -> Uuid,
        org_id -> Nullable<Uuid>,
        bot_id -> Nullable<Uuid>,
        branch_id -> Uuid,
        name -> Varchar,
        description -> Nullable<Text>,
        parent_id -> Nullable<Uuid>,
        slug -> Nullable<Varchar>,
        image_url -> Nullable<Text>,
        sort_order -> Int4,
        is_active -> Bool,
        created_at -> Timestamptz,
        display_order -> Nullable<Int4>,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    price_lists (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        currency -> Nullable<Varchar>,
        is_active -> Nullable<Bool>,
        valid_from -> Nullable<Date>,
        valid_until -> Nullable<Date>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        description -> Nullable<Text>,
        is_default -> Bool,
        customer_group -> Nullable<Varchar>,
        discount_percent -> Numeric,
        org_id -> Uuid,
        bot_id -> Uuid,
    }
}

diesel::table! {
    price_list_items (id) {
        id -> Uuid,
        price_list_id -> Uuid,
        product_id -> Nullable<Uuid>,
        service_id -> Nullable<Uuid>,
        price -> Numeric,
        min_quantity -> Int4,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    inventory_movements (id) {
        id -> Uuid,
        branch_id -> Uuid,
        product_id -> Uuid,
        quantity -> Int4,
        movement_type -> Varchar,
        reference -> Nullable<Varchar>,
        notes -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        reference_type -> Nullable<Varchar>,
        reference_id -> Nullable<Uuid>,
        created_by -> Nullable<Uuid>,
    }
}

diesel::table! {
    product_variants (id) {
        id -> Uuid,
        product_id -> Uuid,
        sku -> Nullable<Varchar>,
        name -> Varchar,
        price_adjustment -> Numeric,
        stock_quantity -> Int4,
        attributes -> Jsonb,
        is_active -> Bool,
        created_at -> Timestamptz,
        global_trade_number -> Nullable<Varchar>,
        net_weight -> Nullable<Numeric>,
        gross_weight -> Nullable<Numeric>,
        width -> Nullable<Numeric>,
        height -> Nullable<Numeric>,
        length -> Nullable<Numeric>,
        color -> Nullable<Varchar>,
        size -> Nullable<Varchar>,
        images -> Nullable<Jsonb>,
    }
}

diesel::table! {
    billing_usage_alerts (id) {
        id -> Uuid,
        branch_id -> Uuid,
        threshold -> Numeric,
        metric -> Varchar,
        recipients -> Text,
        is_active -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        severity -> Varchar,
        current_usage -> Int8,
        usage_limit -> Int8,
        percentage -> Numeric,
        message -> Text,
        acknowledged_at -> Nullable<Timestamptz>,
        acknowledged_by -> Nullable<Uuid>,
        notification_sent -> Bool,
        notification_channels -> Jsonb,
    }
}

diesel::table! {
    billing_alert_history (id) {
        id -> Uuid,
        branch_id -> Uuid,
        alert_id -> Nullable<Uuid>,
        metric -> Varchar,
        severity -> Varchar,
        current_usage -> Int8,
        usage_limit -> Int8,
        percentage -> Numeric,
        message -> Text,
        acknowledged_at -> Nullable<Timestamptz>,
        acknowledged_by -> Nullable<Uuid>,
        resolved_at -> Nullable<Timestamptz>,
        resolution_type -> Nullable<Varchar>,
        created_at -> Timestamptz,
        org_id -> Uuid,
        bot_id -> Uuid,
    }
}

diesel::table! {
    billing_notification_preferences (id) {
        id -> Uuid,
        branch_id -> Uuid,
        enabled -> Nullable<Bool>,
        channels -> Jsonb,
        email_recipients -> Jsonb,
        webhook_url -> Nullable<Text>,
        webhook_secret -> Nullable<Text>,
        slack_webhook_url -> Nullable<Text>,
        teams_webhook_url -> Nullable<Text>,
        sms_numbers -> Jsonb,
        min_severity -> Varchar,
        quiet_hours_start -> Nullable<Int4>,
        quiet_hours_end -> Nullable<Int4>,
        quiet_hours_timezone -> Nullable<Varchar>,
        quiet_hours_days -> Nullable<Jsonb>,
        metric_overrides -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        org_id -> Uuid,
        bot_id -> Uuid,
    }
}

diesel::table! {
    billing_grace_periods (id) {
        id -> Uuid,
        branch_id -> Uuid,
        subscription_id -> Nullable<Uuid>,
        starts_at -> Timestamptz,
        ends_at -> Timestamptz,
        reason -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        metric -> Varchar,
        started_at -> Timestamptz,
        expires_at -> Timestamptz,
        overage_at_start -> Numeric,
        current_overage -> Numeric,
        max_allowed_overage -> Numeric,
        is_active -> Bool,
        ended_at -> Nullable<Timestamptz>,
        end_reason -> Nullable<Varchar>,
    }
}

diesel::joinable!(products -> branches (branch_id));
diesel::joinable!(services -> branches (branch_id));
diesel::joinable!(product_categories -> branches (branch_id));
diesel::joinable!(price_lists -> branches (branch_id));
diesel::joinable!(price_list_items -> price_lists (price_list_id));
diesel::joinable!(price_list_items -> products (product_id));
diesel::joinable!(price_list_items -> services (service_id));
diesel::joinable!(inventory_movements -> branches (branch_id));
diesel::joinable!(inventory_movements -> products (product_id));
diesel::joinable!(product_variants -> products (product_id));

diesel::joinable!(billing_invoices -> branches (branch_id));
diesel::joinable!(billing_invoice_items -> billing_invoices (invoice_id));
diesel::joinable!(billing_payments -> branches (branch_id));
diesel::joinable!(billing_payments -> billing_invoices (invoice_id));
diesel::joinable!(billing_quotes -> branches (branch_id));
diesel::joinable!(billing_quote_items -> billing_quotes (quote_id));
diesel::joinable!(billing_recurring -> branches (branch_id));
diesel::joinable!(billing_tax_rates -> branches (branch_id));

diesel::allow_tables_to_appear_in_same_query!(
    billing_invoices,
    billing_payments,
    billing_quotes,
    billing_recurring,
    billing_tax_rates,
    products,
    price_lists,
    billing_alert_history,
    billing_notification_preferences,
);

diesel::joinable!(billing_usage_alerts -> branches (branch_id));
diesel::joinable!(billing_alert_history -> branches (branch_id));
diesel::joinable!(billing_notification_preferences -> branches (branch_id));
diesel::joinable!(billing_grace_periods -> branches (branch_id));

// Moved verbatim from `botproducts/src/schema.rs` (#1497) — column order
// preserved, so the consuming crate's `Queryable` structs are unaffected.
// Verified against the database by `scripts/schema_db_audit.py` (#1519).
diesel::table! {
    product_prices (id) {
        id -> Uuid,
        price_list_id -> Uuid,
        product_id -> Uuid,
        variation_id -> Nullable<Uuid>,
        price -> Numeric,
        min_quantity -> Numeric,
    }
}
