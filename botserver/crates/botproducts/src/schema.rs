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

pub use botschema::inventory_movements;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::product_variants;

diesel::table! {
    organizations (id) {
        id -> Uuid,
    }
}

// Canonical definition lives in `botschema` (#1497).
pub use botschema::bots;

diesel::allow_tables_to_appear_in_same_query!(
    products,
    services,
    product_categories,
    price_lists,
    organizations,
    product_variations,
    product_stock,
    product_price_lists,
    product_prices,
    product_promotions,
    pos_sessions,
    pos_sales,
);

diesel::table! {
    product_variations (id) {
        id -> Uuid,
        branch_id -> Uuid,
        product_id -> Uuid,
        name -> Varchar,
        sku -> Varchar,
        price -> Nullable<Numeric>,
        attributes -> Nullable<Jsonb>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        cost_price -> Nullable<Numeric>,
        barcode -> Nullable<Text>,
        weight -> Nullable<Numeric>,
        is_active -> Bool,
    }
}

diesel::table! {
    product_stock (id) {
        id -> Uuid,
        branch_id -> Uuid,
        product_id -> Uuid,
        quantity -> Int4,
        min_quantity -> Nullable<Int4>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        variation_id -> Nullable<Uuid>,
        reserved -> Numeric,
        reorder_point -> Nullable<Numeric>,
    }
}

diesel::table! {
    product_price_lists (id) {
        id -> Uuid,
        branch_id -> Uuid,
        product_id -> Uuid,
        price_list_id -> Uuid,
        price -> Numeric,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        name -> Text,
        currency -> Text,
        is_default -> Bool,
        valid_from -> Nullable<Date>,
        valid_until -> Nullable<Date>,
    }
}

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

diesel::table! {
    product_promotions (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        product_id -> Nullable<Uuid>,
        discount_type -> Varchar,
        discount_value -> Numeric,
        starts_at -> Nullable<Timestamptz>,
        ends_at -> Nullable<Timestamptz>,
        is_active -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        product_ids -> Nullable<Jsonb>,
        min_purchase -> Nullable<Numeric>,
        valid_from -> Date,
        valid_until -> Date,
    }
}

diesel::table! {
    pos_sessions (id) {
        id -> Uuid,
        branch_id -> Uuid,
        session_number -> Varchar,
        status -> Nullable<Varchar>,
        opened_at -> Timestamptz,
        closed_at -> Nullable<Timestamptz>,
        opened_by -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        operator_id -> Uuid,
        started_at -> Timestamptz,
        opening_amount -> Numeric,
        closing_amount -> Nullable<Numeric>,
    }
}

diesel::table! {
    pos_sales (id) {
        id -> Uuid,
        branch_id -> Uuid,
        session_id -> Nullable<Uuid>,
        sale_number -> Varchar,
        items -> Nullable<Jsonb>,
        total -> Nullable<Numeric>,
        payment_method -> Nullable<Varchar>,
        status -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        subtotal -> Numeric,
        discount -> Nullable<Numeric>,
        tax -> Nullable<Numeric>,
        nfse_id -> Nullable<Uuid>,
    }
}
