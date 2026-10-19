// Canonical definition lives in `botschema` (#1497).
pub use botschema::billing_invoices;

pub use botschema::billing_invoice_items;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::billing_payments;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::billing_quotes;

pub use botschema::billing_quote_items;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::billing_recurring;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::billing_tax_rates;

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

// Canonical definition lives in `botschema` (#1497).
pub use botschema::services;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::product_categories;

pub use botschema::price_lists;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::price_list_items;

pub use botschema::inventory_movements;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::product_variants;

pub use botschema::billing_usage_alerts;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::billing_alert_history;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::billing_notification_preferences;

pub use botschema::billing_grace_periods;

// The `allow_tables_to_appear_in_same_query!` list moved to `botschema::billing`:
// both sides of every pair are foreign types here, so the orphan rule blocks it.

diesel::table! {
    organizations (id) {
        id -> Uuid,
    }
}

// Canonical definition lives in `botschema` (#1497).
pub use botschema::bots;
