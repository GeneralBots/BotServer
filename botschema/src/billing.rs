use crate::core::{branches};

diesel::table! {
    billing_invoices (id) {
        id -> Uuid,
        branch_id -> Uuid,
        invoice_number -> Varchar,
        customer_id -> Nullable<Uuid>,
        customer_name -> Varchar,
        customer_email -> Nullable<Varchar>,
        customer_address -> Nullable<Text>,
        status -> Varchar,
        issue_date -> Date,
        due_date -> Date,
        subtotal -> Numeric,
        tax_rate -> Numeric,
        tax_amount -> Numeric,
        discount_percent -> Numeric,
        discount_amount -> Numeric,
        total -> Numeric,
        amount_paid -> Numeric,
        amount_due -> Numeric,
        currency -> Varchar,
        notes -> Nullable<Text>,
        terms -> Nullable<Text>,
        footer -> Nullable<Text>,
        paid_at -> Nullable<Timestamptz>,
        sent_at -> Nullable<Timestamptz>,
        voided_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
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
        currency -> Varchar,
        payment_method -> Varchar,
        payment_reference -> Nullable<Varchar>,
        status -> Varchar,
        payer_name -> Nullable<Varchar>,
        payer_email -> Nullable<Varchar>,
        notes -> Nullable<Text>,
        paid_at -> Timestamptz,
        refunded_at -> Nullable<Timestamptz>,
        refund_amount -> Nullable<Numeric>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    billing_quotes (id) {
        id -> Uuid,
        branch_id -> Uuid,
        quote_number -> Varchar,
        customer_id -> Nullable<Uuid>,
        customer_name -> Varchar,
        customer_email -> Nullable<Varchar>,
        customer_address -> Nullable<Text>,
        status -> Varchar,
        issue_date -> Date,
        valid_until -> Date,
        subtotal -> Numeric,
        tax_rate -> Numeric,
        tax_amount -> Numeric,
        discount_percent -> Numeric,
        discount_amount -> Numeric,
        total -> Numeric,
        currency -> Varchar,
        notes -> Nullable<Text>,
        terms -> Nullable<Text>,
        accepted_at -> Nullable<Timestamptz>,
        rejected_at -> Nullable<Timestamptz>,
        converted_invoice_id -> Nullable<Uuid>,
        sent_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
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
        status -> Varchar,
        frequency -> Varchar,
        interval_count -> Int4,
        amount -> Numeric,
        currency -> Varchar,
        description -> Nullable<Text>,
        next_invoice_date -> Date,
        last_invoice_date -> Nullable<Date>,
        last_invoice_id -> Nullable<Uuid>,
        start_date -> Date,
        end_date -> Nullable<Date>,
        invoices_generated -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
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
        is_default -> Bool,
        is_active -> Bool,
        created_at -> Timestamptz,
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

diesel::table! {
    services (id) {
        id -> Uuid,
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
    }
}

diesel::table! {
    product_categories (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        description -> Nullable<Text>,
        parent_id -> Nullable<Uuid>,
        slug -> Nullable<Varchar>,
        image_url -> Nullable<Text>,
        sort_order -> Int4,
        is_active -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    price_lists (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        description -> Nullable<Text>,
        currency -> Varchar,
        is_default -> Bool,
        valid_from -> Nullable<Date>,
        valid_until -> Nullable<Date>,
        customer_group -> Nullable<Varchar>,
        discount_percent -> Numeric,
        is_active -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
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
        alert_id -> Uuid,
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
    }
}

diesel::table! {
    billing_notification_preferences (id) {
        id -> Uuid,
        branch_id -> Uuid,
        enabled -> Bool,
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

diesel::joinable!(billing_usage_alerts -> branches (branch_id));
diesel::joinable!(billing_alert_history -> branches (branch_id));
diesel::joinable!(billing_notification_preferences -> branches (branch_id));
diesel::joinable!(billing_grace_periods -> branches (branch_id));
