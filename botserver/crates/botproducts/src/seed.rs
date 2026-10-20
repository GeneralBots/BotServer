use diesel::prelude::*;
use diesel::PgConnection;
use uuid::Uuid;
use bigdecimal::BigDecimal;
use std::str::FromStr;
use serde_json::json;
use crate::ProductCategory;
use crate::schema::product_categories;
use crate::schema::products;

fn bd(val: f64) -> BigDecimal {
    BigDecimal::from_str(&val.to_string()).unwrap_or_else(|_| BigDecimal::from(0))
}

pub fn seed_default_products(conn: &mut PgConnection, branch_id: Uuid) {
    use diesel::dsl::exists;
    use diesel::select;

    let has_products: bool = select(exists(
        products::table.filter(products::branch_id.eq(branch_id))
    ))
    .get_result(conn)
    .unwrap_or(false);

    if has_products {
        log::info!("Products already seeded for branch {branch_id}, skipping");
        return;
    }

    let categories = seed_categories(conn, branch_id);
    seed_plan_products(conn, branch_id, &categories);
    seed_infra_products(conn, branch_id, &categories);
    seed_comms_products(conn, branch_id, &categories);
    seed_llm_products(conn, branch_id, &categories);

    log::info!("Seeded default cloud catalog products for branch {branch_id}");
}

fn seed_categories(conn: &mut PgConnection, branch_id: Uuid) -> Vec<ProductCategory> {
    let cats = vec![
        ("Plans", "Subscription plans for the SaaS platform", "plans", 1),
        ("VMs", "Virtual machines and compute instances", "vms", 2),
        ("GPU", "GPU-accelerated compute instances", "gpu", 3),
        ("Storage", "Persistent storage volumes", "storage", 4),
        ("Numbers", "Phone numbers (local and toll-free)", "numbers", 5),
        ("Domains", "Custom domain registration and renewal", "domains", 6),
        ("LLM Tokens", "Language model token packages", "llm-tokens", 7),
    ];

    let mut inserted = Vec::new();
    for (name, description, slug, sort_order) in cats {
        let cat = ProductCategory {
            id: Uuid::new_v4(),
            org_id: None,
            bot_id: None,
            branch_id,
            name: name.to_string(),
            description: Some(description.to_string()),
            parent_id: None,
            slug: Some(slug.to_string()),
            image_url: None,
            sort_order,
            is_active: true,
            created_at: chrono::Utc::now(),
            display_order: None,
            updated_at: chrono::Utc::now(),
        };
        match diesel::insert_into(product_categories::table)
            .values(&cat)
            .execute(conn)
        {
            Ok(_) => inserted.push(cat),
            Err(e) => log::warn!("Failed to seed category {name}: {e}"),
        }
    }
    inserted
}

fn find_category_id(categories: &[ProductCategory], slug: &str) -> Option<Uuid> {
    categories
        .iter()
        .find(|c| c.slug.as_deref() == Some(slug))
        .map(|c| c.id)
}

fn seed_plan_products(conn: &mut PgConnection, branch_id: Uuid, categories: &[ProductCategory]) {
    let cat_id = find_category_id(categories, "plans");
    let now = chrono::Utc::now();
    let plans = vec![
        ("free", "Free", "Free tier with basic features", bd(0.0), bd(0.0), "",
         json!({"is_public": true, "period": "month", "messages_per_day": 10, "storage_mb": 20, "bots": 1, "users": 1})),
        ("shared", "Shared", "Shared cloud plan with unlimited API calls", bd(3.99), bd(0.0), "month",
         json!({"is_public": true, "period": "month", "trial_days": 14, "messages_per_day": -1, "storage_gb": 50, "bots": 5, "users": 5})),
        ("private-cloud", "Private Cloud", "Dedicated private cloud deployment", bd(0.0), bd(0.0), "custom",
         json!({"is_public": true, "period": "custom", "messages_per_day": -1, "storage_gb": -1, "bots": -1, "users": -1, "price_label": "Custom"})),
    ];

    for (sku, name, desc, price, cost, unit, attrs) in plans {
        let product = crate::Product {
            id: Uuid::new_v4(),
            org_id: None,
            bot_id: None,
            sku: Some(sku.to_string()),
            name: name.to_string(),
            description: Some(desc.to_string()),
            category: cat_id.map(|id| id.to_string()),
            product_type: "plan".to_string(),
            price,
            cost: Some(cost),
            currency: "USD".to_string(),
            tax_rate: bd(0.0),
            unit: unit.to_string(),
            stock_quantity: -1,
            low_stock_threshold: 0,
            is_active: true,
            images: json!([]),
            attributes: attrs,
            weight: None,
            dimensions: None,
            barcode: None,
            created_at: now,
            updated_at: now,
            tax_code: None,
            tax_class: None,
            global_trade_number: None,
            net_weight: None,
            gross_weight: None,
            width: None,
            height: None,
            length: None,
            brand: None,
            model: None,
            color: None,
            size: None,
            material: None,
            gender: None,
            expiration_date: None,
            min_stock: None,
            max_stock: None,
            reorder_point: None,
            shipping_cost: None,
            external_id: None,
            external_metadata: None,
            slug: None,
            meta_description: None,
            tags: None,
            branch_id,
            category_id: None,
            is_public: true,
        };
        if let Err(e) = diesel::insert_into(products::table).values(&product).execute(conn) {
            log::warn!("Failed to seed product {name}: {e}");
        }
    }
}

fn seed_infra_products(conn: &mut PgConnection, branch_id: Uuid, categories: &[ProductCategory]) {
    let now = chrono::Utc::now();
    let items = vec![
        ("vps-small", "VPS Small", "1 vCPU, 2 GB RAM, 50 GB SSD", "vms", 9.99, 3.50, "month",
         json!({"is_public": true, "vcpu": 1, "ram_gb": 2, "storage_gb": 50, "storage_type": "ssd"})),
        ("vps-medium", "VPS Medium", "2 vCPU, 4 GB RAM, 100 GB SSD", "vms", 19.99, 7.00, "month",
         json!({"is_public": true, "vcpu": 2, "ram_gb": 4, "storage_gb": 100, "storage_type": "ssd"})),
        ("vps-large", "VPS Large", "4 vCPU, 8 GB RAM, 200 GB SSD", "vms", 39.99, 14.00, "month",
         json!({"is_public": true, "vcpu": 4, "ram_gb": 8, "storage_gb": 200, "storage_type": "ssd"})),
        ("gpu-basic", "GPU Basic", "1x RTX 3060, 8 GB VRAM, 50 GB SSD", "gpu", 39.99, 15.00, "month",
         json!({"is_public": true, "gpu_model": "RTX 3060", "vram_gb": 8, "vcpu": 4, "ram_gb": 16})),
        ("gpu-advanced", "GPU Advanced", "1x RTX 4090, 24 GB VRAM, 100 GB SSD", "gpu", 99.99, 40.00, "month",
         json!({"is_public": true, "gpu_model": "RTX 4090", "vram_gb": 24, "vcpu": 8, "ram_gb": 32})),
        ("storage-50", "Storage 50 GB", "50 GB persistent block storage", "storage", 9.99, 3.00, "month",
         json!({"is_public": true, "size_gb": 50, "type": "block"})),
        ("storage-200", "Storage 200 GB", "200 GB persistent block storage", "storage", 29.99, 9.00, "month",
         json!({"is_public": true, "size_gb": 200, "type": "block"})),
        ("storage-1000", "Storage 1 TB", "1 TB persistent block storage", "storage", 99.99, 30.00, "month",
         json!({"is_public": true, "size_gb": 1000, "type": "block"})),
    ];

    for (sku, name, desc, cat_slug, price, cost, unit, attrs) in items {
        let cat_id = find_category_id(categories, cat_slug);
        let product = crate::Product {
            id: Uuid::new_v4(),
            org_id: None,
            bot_id: None,
            sku: Some(sku.to_string()),
            name: name.to_string(),
            description: Some(desc.to_string()),
            category: cat_id.map(|id| id.to_string()),
            product_type: "infrastructure".to_string(),
            price: bd(price),
            cost: Some(bd(cost)),
            currency: "USD".to_string(),
            tax_rate: bd(0.0),
            unit: unit.to_string(),
            stock_quantity: -1,
            low_stock_threshold: 0,
            is_active: true,
            images: json!([]),
            attributes: attrs,
            weight: None,
            dimensions: None,
            barcode: None,
            created_at: now,
            updated_at: now,
            tax_code: None,
            tax_class: None,
            global_trade_number: None,
            net_weight: None,
            gross_weight: None,
            width: None,
            height: None,
            length: None,
            brand: None,
            model: None,
            color: None,
            size: None,
            material: None,
            gender: None,
            expiration_date: None,
            min_stock: None,
            max_stock: None,
            reorder_point: None,
            shipping_cost: None,
            external_id: None,
            external_metadata: None,
            slug: None,
            meta_description: None,
            tags: None,
            branch_id,
            category_id: None,
            is_public: true,
        };
        if let Err(e) = diesel::insert_into(products::table).values(&product).execute(conn) {
            log::warn!("Failed to seed product {name}: {e}");
        }
    }
}

fn seed_comms_products(conn: &mut PgConnection, branch_id: Uuid, categories: &[ProductCategory]) {
    let now = chrono::Utc::now();
    let items = vec![
        ("number-local", "Local Phone Number", "Local phone number in your area code", "numbers", 5.99, 2.00, "month",
         json!({"is_public": true, "type": "local", "sms": true, "voice": true, "whatsapp": true})),
        ("number-tollfree", "Toll-Free Number", "Toll-free 1-800 number", "numbers", 9.99, 4.00, "month",
         json!({"is_public": true, "type": "tollfree", "sms": true, "voice": true, "whatsapp": true})),
        ("domain-com", ".com Domain", "Custom .com domain registration (per year)", "domains", 21.99, 10.00, "year",
         json!({"is_public": true, "tld": "com", "renewal_price": 21.99})),
        ("domain-org", ".org Domain", "Custom .org domain registration (per year)", "domains", 19.99, 9.00, "year",
         json!({"is_public": true, "tld": "org", "renewal_price": 19.99})),
    ];

    for (sku, name, desc, cat_slug, price, cost, unit, attrs) in items {
        let cat_id = find_category_id(categories, cat_slug);
        let product = crate::Product {
    id: Uuid::new_v4(),
    org_id: None,
    bot_id: None,
    sku: Some(sku.to_string()),
    name: name.to_string(),
    description: Some(desc.to_string()),
    category: cat_id.map(|id| id.to_string()),
    product_type: "communication".to_string(),
    price: bd(price),
    cost: Some(bd(cost)),
    currency: "USD".to_string(),
    tax_rate: bd(0.0),
    unit: unit.to_string(),
    stock_quantity: -1,
    low_stock_threshold: 0,
    is_active: true,
    images: json!([]),
    attributes: attrs,
    weight: None,
    dimensions: None,
    barcode: None,
    created_at: now,
    updated_at: now,
    tax_code: None,
    tax_class: None,
    global_trade_number: None,
    net_weight: None,
    gross_weight: None,
    width: None,
    height: None,
    length: None,
    brand: None,
    model: None,
    color: None,
    size: None,
    material: None,
    gender: None,
    expiration_date: None,
    min_stock: None,
    max_stock: None,
    reorder_point: None,
    shipping_cost: None,
    external_id: None,
    external_metadata: None,
    slug: None,
    meta_description: None,
    tags: None,
    branch_id,
    category_id: None,
    is_public: true,
};
        if let Err(e) = diesel::insert_into(products::table).values(&product).execute(conn) {
            log::warn!("Failed to seed product {name}: {e}");
        }
    }
}

fn seed_llm_products(conn: &mut PgConnection, branch_id: Uuid, categories: &[ProductCategory]) {
    let now = chrono::Utc::now();
    let items = vec![
        ("llm-tokens-1m", "1M LLM Tokens", "1 million language model tokens", "llm-tokens", 9.99, 2.00, "one-time",
         json!({"is_public": true, "tokens": 1_000_000, "models": ["gpt-4", "claude-3", "llama-3"]})),
        ("llm-tokens-10m", "10M LLM Tokens", "10 million language model tokens", "llm-tokens", 79.99, 16.00, "one-time",
         json!({"is_public": true, "tokens": 10_000_000, "models": ["gpt-4", "claude-3", "llama-3"]})),
    ];

    for (sku, name, desc, cat_slug, price, cost, unit, attrs) in items {
        let cat_id = find_category_id(categories, cat_slug);
        let product = crate::Product {
    id: Uuid::new_v4(),
    org_id: None,
    bot_id: None,
    sku: Some(sku.to_string()),
    name: name.to_string(),
    description: Some(desc.to_string()),
    category: cat_id.map(|id| id.to_string()),
    product_type: "llm-tokens".to_string(),
    price: bd(price),
    cost: Some(bd(cost)),
    currency: "USD".to_string(),
    tax_rate: bd(0.0),
    unit: unit.to_string(),
    stock_quantity: -1,
    low_stock_threshold: 0,
    is_active: true,
    images: json!([]),
    attributes: attrs,
    weight: None,
    dimensions: None,
    barcode: None,
    created_at: now,
    updated_at: now,
    tax_code: None,
    tax_class: None,
    global_trade_number: None,
    net_weight: None,
    gross_weight: None,
    width: None,
    height: None,
    length: None,
    brand: None,
    model: None,
    color: None,
    size: None,
    material: None,
    gender: None,
    expiration_date: None,
    min_stock: None,
    max_stock: None,
    reorder_point: None,
    shipping_cost: None,
    external_id: None,
    external_metadata: None,
    slug: None,
    meta_description: None,
    tags: None,
    branch_id,
    category_id: None,
    is_public: true,
};
        if let Err(e) = diesel::insert_into(products::table).values(&product).execute(conn) {
            log::warn!("Failed to seed product {name}: {e}");
        }
    }
}
