

pub mod inventory;
pub mod scope;
pub mod pos;
pub mod pricing;
pub mod routes;
pub mod schema;
pub mod seed;
pub mod service_tax;

use axum::{
    http::HeaderMap,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post, put},
    Json, Router,
};

use bigdecimal::{BigDecimal, ToPrimitive};
use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use crate::schema::{
    inventory_movements, price_list_items, price_lists, product_categories, product_variants,
    products, services,
};

pub type DbPool = r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>;

pub type GetDefaultBotFn = fn(&mut diesel::PgConnection) -> Uuid;

pub struct ProductsState {
    pub pool: Arc<DbPool>,
    pub get_default_bot: Option<GetDefaultBotFn>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = products)]
pub struct Product {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub name: String,
    pub sku: String,
    pub description: Option<String>,
    pub price: Option<BigDecimal>,
    pub currency: Option<String>,
    pub stock_quantity: Option<i32>,
    pub category_id: Option<Uuid>,
    pub attributes: Option<serde_json::Value>,
    pub is_public: Option<bool>,
    pub is_active: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub category: Option<String>,
    pub product_type: String,
    pub cost: Option<BigDecimal>,
    pub tax_rate: BigDecimal,
    pub unit: String,
    pub low_stock_threshold: i32,
    pub images: serde_json::Value,
    pub weight: Option<BigDecimal>,
    pub dimensions: Option<serde_json::Value>,
    pub barcode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = services)]
pub struct Service {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub name: String,
    pub sku: String,
    pub description: Option<String>,
    pub price: Option<BigDecimal>,
    pub currency: Option<String>,
    pub is_recurring: Option<bool>,
    pub billing_cycle: Option<String>,
    pub attributes: Option<serde_json::Value>,
    pub is_active: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub category: Option<String>,
    pub service_type: String,
    pub hourly_rate: Option<BigDecimal>,
    pub fixed_price: Option<BigDecimal>,
    pub duration_minutes: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = product_categories)]
pub struct ProductCategory {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub display_order: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub image_url: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = price_lists)]
pub struct PriceList {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub name: String,
    pub currency: Option<String>,
    pub is_active: Option<bool>,
    pub valid_from: Option<NaiveDate>,
    pub valid_until: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub description: Option<String>,
    pub is_default: bool,
    pub customer_group: Option<String>,
    pub discount_percent: BigDecimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = price_list_items)]
pub struct PriceListItem {
    pub id: Uuid,
    pub price_list_id: Uuid,
    pub product_id: Option<Uuid>,
    pub service_id: Option<Uuid>,
    pub price: BigDecimal,
    pub min_quantity: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = inventory_movements)]
pub struct InventoryMovement {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub product_id: Uuid,
    pub quantity: i32,
    pub movement_type: String,
    pub reference: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub reference_type: Option<String>,
    pub reference_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = product_variants)]
pub struct ProductVariant {
    pub id: Uuid,
    pub product_id: Uuid,
    pub sku: Option<String>,
    pub name: String,
    pub price_adjustment: BigDecimal,
    pub stock_quantity: i32,
    pub attributes: serde_json::Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub global_trade_number: Option<String>,
    pub net_weight: Option<BigDecimal>,
    pub gross_weight: Option<BigDecimal>,
    pub width: Option<BigDecimal>,
    pub height: Option<BigDecimal>,
    pub length: Option<BigDecimal>,
    pub color: Option<String>,
    pub size: Option<String>,
    pub images: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProductRequest {
    pub name: String,
    pub description: Option<String>,
    pub sku: Option<String>,
    pub category: Option<String>,
    pub product_type: Option<String>,
    pub price: f64,
    pub cost: Option<f64>,
    pub currency: Option<String>,
    pub tax_rate: Option<f64>,
    pub unit: Option<String>,
    pub stock_quantity: Option<i32>,
    pub low_stock_threshold: Option<i32>,
    pub images: Option<Vec<String>>,
    pub barcode: Option<String>,
    pub weight: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProductRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub sku: Option<String>,
    pub category: Option<String>,
    pub price: Option<f64>,
    pub cost: Option<f64>,
    pub tax_rate: Option<f64>,
    pub unit: Option<String>,
    pub stock_quantity: Option<i32>,
    pub low_stock_threshold: Option<i32>,
    pub is_active: Option<bool>,
    pub barcode: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateServiceRequest {
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub service_type: Option<String>,
    pub hourly_rate: Option<f64>,
    pub fixed_price: Option<f64>,
    pub currency: Option<String>,
    pub duration_minutes: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateServiceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub hourly_rate: Option<f64>,
    pub fixed_price: Option<f64>,
    pub duration_minutes: Option<i32>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub slug: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePriceListRequest {
    pub name: String,
    pub description: Option<String>,
    pub currency: Option<String>,
    pub discount_percent: Option<f64>,
    pub customer_group: Option<String>,
    pub valid_from: Option<String>,
    pub valid_until: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdjustStockRequest {
    pub quantity: i32,
    pub movement_type: String,
    pub notes: Option<String>,
    pub reference_type: Option<String>,
    pub reference_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub search: Option<String>,
    pub category: Option<String>,
    pub is_active: Option<bool>,
    pub low_stock: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ProductQuery {
    pub category: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProductStats {
    pub total_products: i64,
    pub active_products: i64,
    pub total_services: i64,
    pub active_services: i64,
    pub low_stock_count: i64,
    pub total_stock_value: f64,
    pub categories_count: i64,
    pub price_lists_count: i64,
}

#[derive(Debug, Serialize)]
pub struct ProductWithVariants {
    pub product: Product,
    pub variants: Vec<ProductVariant>,
}

mod lib_part1;
mod lib_part2;
mod lib_part3;
mod lib_part4;
mod lib_part5;
mod lib_part6;
mod lib_part7;
mod lib_part8;

pub use lib_part1::*;
use lib_part2::*;
pub use lib_part3::*;
pub use lib_part4::*;
pub use lib_part5::*;
use lib_part6::*;
use lib_part7::*;
pub use lib_part8::*;
