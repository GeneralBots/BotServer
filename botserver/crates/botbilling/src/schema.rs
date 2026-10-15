diesel::table! {
    billing_invoices (id) {
        id -> Uuid,
        branch_id -> Uuid,
        invoice_number -> Varchar,
        customer_name -> Nullable<Varchar>,
        customer_email -> Nullable<Varchar>,
        status -> Nullable<Varchar>,
        total -> Nullable<Numeric>,
        currency -> Nullable<Varchar>,
        due_date -> Nullable<Date>,
        paid_at -> Nullable<Timestamptz>,
        notes -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        customer_id -> Nullable<Uuid>,
        customer_address -> Nullable<Text>,
        issue_date -> Date,
        subtotal -> Numeric,
        tax_rate -> Numeric,
        tax_amount -> Numeric,
        discount_percent -> Numeric,
        discount_amount -> Numeric,
        amount_paid -> Numeric,
        amount_due -> Numeric,
        terms -> Nullable<Text>,
        footer -> Nullable<Text>,
        sent_at -> Nullable<Timestamptz>,
        voided_at -> Nullable<Timestamptz>,
    }
}

pub use botschema::billing_invoice_items;

diesel::table! {
    billing_payments (id) {
        id -> Uuid,
        branch_id -> Uuid,
        invoice_id -> Nullable<Uuid>,
        amount -> Numeric,
        currency -> Nullable<Varchar>,
        payment_method -> Nullable<Varchar>,
        status -> Nullable<Varchar>,
        paid_at -> Nullable<Timestamptz>,
        gateway_response -> Nullable<Jsonb>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        payment_number -> Varchar,
        payment_reference -> Nullable<Varchar>,
        payer_name -> Nullable<Varchar>,
        payer_email -> Nullable<Varchar>,
        notes -> Nullable<Text>,
        refunded_at -> Nullable<Timestamptz>,
        refund_amount -> Nullable<Numeric>,
    }
}

diesel::table! {
    billing_quotes (id) {
        id -> Uuid,
        branch_id -> Uuid,
        quote_number -> Varchar,
        customer_name -> Nullable<Varchar>,
        customer_email -> Nullable<Varchar>,
        items -> Nullable<Jsonb>,
        total -> Nullable<Numeric>,
        currency -> Nullable<Varchar>,
        status -> Nullable<Varchar>,
        valid_until -> Nullable<Date>,
        notes -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        customer_id -> Nullable<Uuid>,
        customer_address -> Nullable<Text>,
        issue_date -> Date,
        subtotal -> Numeric,
        tax_rate -> Numeric,
        tax_amount -> Numeric,
        discount_percent -> Numeric,
        discount_amount -> Numeric,
        terms -> Nullable<Text>,
        accepted_at -> Nullable<Timestamptz>,
        rejected_at -> Nullable<Timestamptz>,
        converted_invoice_id -> Nullable<Uuid>,
        sent_at -> Nullable<Timestamptz>,
    }
}

pub use botschema::billing_quote_items;

diesel::table! {
    billing_recurring (id) {
        id -> Uuid,
        branch_id -> Uuid,
        plan_name -> Varchar,
        amount -> Nullable<Numeric>,
        currency -> Nullable<Varchar>,
        status -> Nullable<Varchar>,
        trial_end -> Nullable<Timestamptz>,
        current_period_start -> Nullable<Timestamptz>,
        current_period_end -> Nullable<Timestamptz>,
        cancelled_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        customer_id -> Nullable<Uuid>,
        customer_name -> Varchar,
        customer_email -> Nullable<Varchar>,
        frequency -> Varchar,
        interval_count -> Int4,
        description -> Nullable<Text>,
        next_invoice_date -> Date,
        last_invoice_date -> Nullable<Date>,
        last_invoice_id -> Nullable<Uuid>,
        start_date -> Date,
        end_date -> Nullable<Date>,
        invoices_generated -> Int4,
    }
}

diesel::table! {
    billing_tax_rates (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        rate -> Numeric,
        country -> Nullable<Varchar>,
        region -> Nullable<Varchar>,
        is_default -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        description -> Nullable<Text>,
        is_active -> Bool,
    }
}

diesel::table! {
    products (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        sku -> Varchar,
        description -> Nullable<Text>,
        price -> Nullable<Numeric>,
        currency -> Nullable<Varchar>,
        stock_quantity -> Nullable<Int4>,
        category_id -> Nullable<Uuid>,
        attributes -> Nullable<Jsonb>,
        is_public -> Nullable<Bool>,
        is_active -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        category -> Nullable<Varchar>,
        product_type -> Varchar,
        cost -> Nullable<Numeric>,
        tax_rate -> Numeric,
        unit -> Varchar,
        low_stock_threshold -> Int4,
        images -> Jsonb,
        weight -> Nullable<Numeric>,
        dimensions -> Nullable<Jsonb>,
        barcode -> Nullable<Varchar>,
    }
}

diesel::table! {
    services (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        sku -> Varchar,
        description -> Nullable<Text>,
        price -> Nullable<Numeric>,
        currency -> Nullable<Varchar>,
        is_recurring -> Nullable<Bool>,
        billing_cycle -> Nullable<Varchar>,
        attributes -> Nullable<Jsonb>,
        is_active -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        category -> Nullable<Varchar>,
        service_type -> Varchar,
        hourly_rate -> Nullable<Numeric>,
        fixed_price -> Nullable<Numeric>,
        duration_minutes -> Nullable<Int4>,
    }
}

diesel::table! {
    product_categories (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        slug -> Varchar,
        description -> Nullable<Text>,
        parent_id -> Nullable<Uuid>,
        display_order -> Nullable<Int4>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        image_url -> Nullable<Text>,
        sort_order -> Int4,
        is_active -> Bool,
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
    }
}

// Canonical definition lives in `botschema` (#1497).
pub use botschema::price_list_items;

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

// Canonical definition lives in `botschema` (#1497).
pub use botschema::product_variants;

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
        value -> Numeric,
        threshold -> Numeric,
        sent_at -> Timestamptz,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        severity -> Varchar,
        current_usage -> Int8,
        usage_limit -> Int8,
        percentage -> Numeric,
        message -> Text,
        acknowledged_at -> Nullable<Timestamptz>,
        acknowledged_by -> Nullable<Uuid>,
        resolved_at -> Nullable<Timestamptz>,
        resolution_type -> Nullable<Varchar>,
    }
}

diesel::table! {
    billing_notification_preferences (id) {
        id -> Uuid,
        branch_id -> Uuid,
        event_type -> Varchar,
        channel -> Varchar,
        enabled -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
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

diesel::allow_tables_to_appear_in_same_query!(
    billing_invoices,
    billing_payments,
    billing_quotes,
    billing_recurring,
    billing_tax_rates,
    products,
    services,
    product_categories,
    price_lists,
    inventory_movements,
    billing_usage_alerts,
    billing_alert_history,
    billing_notification_preferences,
    billing_grace_periods,
);

diesel::table! {
    organizations (id) {
        id -> Uuid,
    }
}

diesel::table! {
    bots (id) {
        id -> Uuid,
        branch_id -> Uuid,
        bot_id -> Uuid,
        name -> Varchar,
        slug -> Varchar,
        org_id -> Uuid,
        tenant_id -> Nullable<Uuid>,
        is_default_for_branch -> Nullable<Bool>,
        description -> Nullable<Text>,
        is_public -> Nullable<Bool>,
        is_active -> Nullable<Bool>,
        avatar_url -> Nullable<Varchar>,
        settings -> Nullable<Jsonb>,
        metadata -> Nullable<Jsonb>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        llm_provider -> Varchar,
        llm_config -> Jsonb,
        context_provider -> Varchar,
        context_config -> Jsonb,
        database_name -> Nullable<Varchar>,
    }
}
