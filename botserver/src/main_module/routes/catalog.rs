use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use bigdecimal::ToPrimitive;
use diesel::prelude::*;
use serde::Serialize;
use std::sync::Arc;
use botcore::shared::state::AppState;

mod prod_schema {
    // Canonical definition lives in `botschema` (#1497).
    pub use botschema::products;
}

#[derive(Debug, Clone, Serialize, Queryable, Selectable)]
#[diesel(table_name = prod_schema::products)]
struct CatalogProduct {
    id: uuid::Uuid,
    name: String,
    sku: Option<String>,
    description: Option<String>,
    price: bigdecimal::BigDecimal,
    currency: String,
    stock_quantity: i32,
    attributes: serde_json::Value,
    is_public: bool,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CatalogProductResponse {
    id: uuid::Uuid,
    sku: String,
    name: String,
    description: Option<String>,
    price_cents: i64,
    price_usd: f64,
    currency: String,
    attributes: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<CatalogProduct> for CatalogProductResponse {
    fn from(p: CatalogProduct) -> Self {
        let price_f64 = p.price.to_f64().unwrap_or(0.0);
        Self {
            id: p.id,
            sku: p.sku.unwrap_or_default(),
            name: p.name,
            description: p.description,
            price_cents: (price_f64 * 100.0).round() as i64,
            price_usd: price_f64,
            currency: p.currency,
            attributes: p.attributes,
            created_at: p.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonLdItemList {
    #[serde(rename = "@context")]
    context: String,
    #[serde(rename = "@type")]
    json_type: String,
    item_list_element: Vec<JsonLdListItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonLdListItem {
    #[serde(rename = "@type")]
    pub json_type: String,
    pub position: u32,
    pub item: JsonLdProduct,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonLdProduct {
    #[serde(rename = "@type")]
    pub json_type: String,
    pub name: String,
    pub description: String,
    pub sku: String,
    pub offers: JsonLdOffer,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonLdOffer {
    #[serde(rename = "@type")]
    json_type: String,
    price: String,
    price_currency: String,
    availability: String,
}

fn load_public_products(
    conn: &mut diesel::PgConnection,
) -> Result<Vec<CatalogProduct>, diesel::result::Error> {
    use self::prod_schema::products::dsl;

    dsl::products
        .filter(dsl::is_active.eq(true))
        .filter(dsl::is_public.eq(true))
        // Public catalog shows only the global (nil-branch) seed; org-specific
        // products are served by the branch-scoped /api/products/* endpoints.
        // Without this filter, every org's signup seed is returned too, which
        // duplicates each product (issue #880).
        .filter(dsl::branch_id.eq(uuid::Uuid::nil()))
        .select(CatalogProduct::as_select())
        .order(dsl::name)
        .load::<CatalogProduct>(conn)
}

pub async fn list_products(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<CatalogProductResponse>>, (StatusCode, String)> {
    let pool = state.conn.clone();
    let mut conn = tokio::task::spawn_blocking(move || pool.get())
        .await
        .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, format!("Join error: {e}")))?
        .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, format!("DB error: {e}")))?;

    let products = load_public_products(&mut conn).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}"))
    })?;

    let public: Vec<CatalogProductResponse> = products
        .into_iter()
        .map(CatalogProductResponse::from)
        .collect();

    Ok(Json(public))
}

pub async fn get_product_by_sku(
    State(state): State<Arc<AppState>>,
    Path(product_sku): Path<String>,
) -> Result<Json<CatalogProductResponse>, (StatusCode, String)> {
    use self::prod_schema::products::dsl;

    let pool = state.conn.clone();
    let mut conn = tokio::task::spawn_blocking(move || pool.get())
        .await
        .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, format!("Join error: {e}")))?
        .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, format!("DB error: {e}")))?;

    let product: Option<CatalogProduct> = dsl::products
        .filter(dsl::is_active.eq(true))
        .filter(dsl::is_public.eq(true))
        .filter(dsl::branch_id.eq(uuid::Uuid::nil()))
        .filter(dsl::sku.eq(&product_sku))
        .select(CatalogProduct::as_select())
        .first(&mut conn)
        .optional()
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}"))
        })?;

    match product {
        Some(p) => Ok(Json(p.into())),
        None => Err((StatusCode::NOT_FOUND, format!("Product '{product_sku}' not found"))),
    }
}

pub async fn list_plans(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<CatalogProductResponse>>, (StatusCode, String)> {
    use self::prod_schema::products::dsl;

    let pool = state.conn.clone();
    let mut conn = tokio::task::spawn_blocking(move || pool.get())
        .await
        .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, format!("Join error: {e}")))?
        .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, format!("DB error: {e}")))?;

    let products: Vec<CatalogProduct> = dsl::products
        .filter(dsl::is_active.eq(true))
        .filter(dsl::is_public.eq(true))
        .filter(dsl::branch_id.eq(uuid::Uuid::nil()))
        .select(CatalogProduct::as_select())
        .order(dsl::price.asc())
        .load(&mut conn)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}"))
        })?;

    let public: Vec<CatalogProductResponse> = products
        .into_iter()
        .map(CatalogProductResponse::from)
        .collect();

    Ok(Json(public))
}

pub async fn prices_json(
    State(state): State<Arc<AppState>>,
) -> Result<Json<JsonLdItemList>, (StatusCode, String)> {
    let pool = state.conn.clone();
    let mut conn = tokio::task::spawn_blocking(move || pool.get())
        .await
        .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, format!("Join error: {e}")))?
        .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, format!("DB error: {e}")))?;

    let products = load_public_products(&mut conn).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}"))
    })?;

    let items: Vec<JsonLdListItem> = products
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            let price_usd = p.price.to_f64().unwrap_or(0.0);
            JsonLdListItem {
                json_type: "ListItem".to_string(),
                position: (i + 1) as u32,
                item: JsonLdProduct {
                    json_type: "Product".to_string(),
                    name: p.name,
                    description: p.description.unwrap_or_default(),
                    sku: p.sku.unwrap_or_default(),
                    offers: JsonLdOffer {
                        json_type: "Offer".to_string(),
                        price: format!("{:.2}", price_usd),
                        price_currency: p.currency,
                        availability: "https://schema.org/InStock".to_string(),
                    },
                },
            }
        })
        .collect();

    Ok(Json(JsonLdItemList {
        context: "https://schema.org".to_string(),
        json_type: "ItemList".to_string(),
        item_list_element: items,
    }))
}

pub fn configure_catalog_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/catalog/products", get(list_products))
        .route("/api/catalog/products/:sku", get(get_product_by_sku))
        .route("/api/catalog/plans", get(list_plans))
        .route("/api/catalog/prices.json", get(prices_json))
}
