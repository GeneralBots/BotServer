use crate::types::{
    CreateItemRequest, CreateMovementRequest, CreatePoRequest, InventoryItem, InventoryMovement,
    PurchaseOrder,
};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

pub use botlib::db_pool::DbPool;

#[derive(Clone)]
pub struct InventoryState {
    pub pool: DbPool,
}

/// Resolves the caller's tenant branch from the server-minted JWT claims
/// (issue #734). Falls back to the global nil branch for anonymous/system
/// callers; every query is still bounded by the resolved branch.
fn resolve_branch(headers: &HeaderMap) -> Uuid {
    botsecurity_core::tenant::branch_from_claims(headers).unwrap_or_else(Uuid::nil)
}

pub fn configure_inventory_routes() -> Router<Arc<InventoryState>> {
    Router::new()
        .route("/api/erp/inventory/items", get(list_items).post(create_item))
        .route("/api/erp/inventory/items/:id", get(get_item))
        .route("/api/erp/inventory/movements", get(list_movements).post(create_movement))
        .route("/api/erp/inventory/purchase-orders", get(list_pos).post(create_po))
        .route("/api/erp/inventory/purchase-orders/:id", get(get_po))
}

async fn list_items(
    State(state): State<Arc<InventoryState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<InventoryItem>>, StatusCode> {
    let branch = resolve_branch(&headers);
    let mut conn = state.pool.get().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let items = diesel::sql_query(
        "SELECT id, branch_id, product_id, sku, name, description, quantity, unit, \
         min_stock, max_stock, location, category, unit_cost, is_active, created_at, updated_at \
         FROM inventory_items WHERE branch_id = $1 ORDER BY name",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load::<ItemDbRow>(&mut conn)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .into_iter()
    .map(item_from_row)
    .collect();
    Ok(Json(items))
}

async fn create_item(
    State(state): State<Arc<InventoryState>>,
    headers: HeaderMap,
    Json(payload): Json<CreateItemRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let branch = resolve_branch(&headers);
    let mut conn = state.pool.get().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let id = Uuid::new_v4();
    let qty = payload.quantity.unwrap_or_default();
    let cost = payload.unit_cost.unwrap_or_default();

    diesel::sql_query(
        "INSERT INTO inventory_items (id, branch_id, sku, name, description, quantity, unit, \
         min_stock, max_stock, location, category, unit_cost, is_active) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, true)",
    )
    .bind::<diesel::sql_types::Uuid, _>(&id)
    .bind::<diesel::sql_types::Uuid, _>(&branch)
    .bind::<diesel::sql_types::Text, _>(&payload.sku)
    .bind::<diesel::sql_types::Text, _>(&payload.name)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&payload.description)
    .bind::<diesel::sql_types::Numeric, _>(&qty)
    .bind::<diesel::sql_types::Text, _>(&payload.unit.unwrap_or_else(|| "unit".to_string()))
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Numeric>, _>(&payload.min_stock)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Numeric>, _>(&payload.max_stock)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&payload.location)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&payload.category)
    .bind::<diesel::sql_types::Numeric, _>(&cost)
    .execute(&mut conn)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({"id": id})))
}

async fn get_item(
    State(state): State<Arc<InventoryState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<InventoryItem>, StatusCode> {
    let branch = resolve_branch(&headers);
    let mut conn = state.pool.get().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let row = diesel::sql_query(
        "SELECT id, branch_id, product_id, sku, name, description, quantity, unit, \
         min_stock, max_stock, location, category, unit_cost, is_active, created_at, updated_at \
         FROM inventory_items WHERE id = $1 AND branch_id = $2",
    )
    .bind::<diesel::sql_types::Uuid, _>(&id)
    .bind::<diesel::sql_types::Uuid, _>(&branch)
    .get_result::<ItemDbRow>(&mut conn)
    .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(item_from_row(row)))
}

async fn list_movements(
    State(state): State<Arc<InventoryState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<InventoryMovement>>, StatusCode> {
    let branch = resolve_branch(&headers);
    let mut conn = state.pool.get().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let rows = diesel::sql_query(
        "SELECT id, branch_id, item_id, movement_type, quantity, reference_type, \
         reference_id, notes, created_by, created_at \
         FROM inventory_movements WHERE branch_id = $1 ORDER BY created_at DESC",
    )
    .bind::<diesel::sql_types::Uuid, _>(&branch)
    .load::<MovementDbRow>(&mut conn)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .into_iter()
    .map(|r| InventoryMovement {
        id: r.id,
        branch_id: r.branch_id,
        item_id: r.item_id,
        movement_type: r.movement_type,
        quantity: r.quantity,
        reference_type: r.reference_type,
        reference_id: r.reference_id,
        notes: r.notes,
        created_by: r.created_by,
        created_at: r.created_at,
    })
    .collect();
    Ok(Json(rows))
}

async fn create_movement(
    State(state): State<Arc<InventoryState>>,
    headers: HeaderMap,
    Json(payload): Json<CreateMovementRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let branch = resolve_branch(&headers);
    let mut conn = state.pool.get().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let id = Uuid::new_v4();

    diesel::sql_query(
        "INSERT INTO inventory_movements (id, branch_id, item_id, movement_type, quantity, notes) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind::<diesel::sql_types::Uuid, _>(&id)
    .bind::<diesel::sql_types::Uuid, _>(&branch)
    .bind::<diesel::sql_types::Uuid, _>(&payload.item_id)
    .bind::<diesel::sql_types::Text, _>(&payload.movement_type)
    .bind::<diesel::sql_types::Numeric, _>(&payload.quantity)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&payload.notes)
    .execute(&mut conn)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({"id": id})))
}

async fn list_pos(
    State(state): State<Arc<InventoryState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<PurchaseOrder>>, StatusCode> {
    let branch = resolve_branch(&headers);
    let mut conn = state.pool.get().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let rows = diesel::sql_query(
        "SELECT id, branch_id, po_number, vendor_name, status, total_amount, currency, \
         expected_date, notes, created_at \
         FROM purchase_orders WHERE branch_id = $1 ORDER BY created_at DESC",
    )
    .bind::<diesel::sql_types::Uuid, _>(&branch)
    .load::<PoDbRow>(&mut conn)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .into_iter()
    .map(|r| PurchaseOrder {
        id: r.id,
        branch_id: r.branch_id,
        po_number: r.po_number,
        vendor_name: r.vendor_name,
        status: r.status,
        total_amount: r.total_amount,
        currency: r.currency,
        expected_date: r.expected_date,
        notes: r.notes,
        created_at: r.created_at,
    })
    .collect();
    Ok(Json(rows))
}

async fn create_po(
    State(state): State<Arc<InventoryState>>,
    headers: HeaderMap,
    Json(payload): Json<CreatePoRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let branch = resolve_branch(&headers);
    let mut conn = state.pool.get().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let po_id = Uuid::new_v4();
    let po_number = format!("PO-{}", chrono::Utc::now().format("%Y%m%d-%06x"));

    conn.transaction(|tx| {
        let mut total = rust_decimal::Decimal::ZERO;
        for item in &payload.items {
            total += item.unit_price * item.quantity;
        }

        diesel::sql_query(
            "INSERT INTO purchase_orders (id, branch_id, po_number, vendor_name, status, total_amount, expected_date, notes) \
             VALUES ($1, $2, $3, $4, 'draft', $5, $6, $7)",
        )
        .bind::<diesel::sql_types::Uuid, _>(&po_id)
        .bind::<diesel::sql_types::Uuid, _>(&branch)
        .bind::<diesel::sql_types::Text, _>(&po_number)
        .bind::<diesel::sql_types::Text, _>(&payload.vendor_name)
        .bind::<diesel::sql_types::Numeric, _>(&total)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Date>, _>(&payload.expected_date)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&payload.notes)
        .execute(tx)?;

        for item in &payload.items {
            diesel::sql_query(
                "INSERT INTO purchase_order_items (id, branch_id, po_id, item_id, description, quantity, unit_price, total_price) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind::<diesel::sql_types::Uuid, _>(&Uuid::new_v4())
            .bind::<diesel::sql_types::Uuid, _>(&branch)
            .bind::<diesel::sql_types::Uuid, _>(&po_id)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Uuid>, _>(&item.item_id)
            .bind::<diesel::sql_types::Text, _>(&item.description)
            .bind::<diesel::sql_types::Numeric, _>(&item.quantity)
            .bind::<diesel::sql_types::Numeric, _>(&item.unit_price)
            .bind::<diesel::sql_types::Numeric, _>(&(item.unit_price * item.quantity))
            .execute(tx)?;
        }

        Ok::<_, diesel::result::Error>(())
    })
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({"id": po_id})))
}

async fn get_po(
    State(state): State<Arc<InventoryState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<PurchaseOrder>, StatusCode> {
    let branch = resolve_branch(&headers);
    let mut conn = state.pool.get().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let row = diesel::sql_query(
        "SELECT id, branch_id, po_number, vendor_name, status, total_amount, currency, \
         expected_date, notes, created_at \
         FROM purchase_orders WHERE id = $1 AND branch_id = $2",
    )
    .bind::<diesel::sql_types::Uuid, _>(&id)
    .bind::<diesel::sql_types::Uuid, _>(&branch)
    .get_result::<PoDbRow>(&mut conn)
    .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(PurchaseOrder {
        id: row.id,
        branch_id: row.branch_id,
        po_number: row.po_number,
        vendor_name: row.vendor_name,
        status: row.status,
        total_amount: row.total_amount,
        currency: row.currency,
        expected_date: row.expected_date,
        notes: row.notes,
        created_at: row.created_at,
    }))
}

fn item_from_row(r: ItemDbRow) -> InventoryItem {
    InventoryItem {
        id: r.id,
        branch_id: r.branch_id,
        product_id: r.product_id,
        sku: r.sku,
        name: r.name,
        description: r.description,
        quantity: r.quantity,
        unit: r.unit,
        min_stock: r.min_stock,
        max_stock: r.max_stock,
        location: r.location,
        category: r.category,
        unit_cost: r.unit_cost,
        is_active: r.is_active,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

#[derive(diesel::QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct ItemDbRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    branch_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
    product_id: Option<Uuid>,
    #[diesel(sql_type = diesel::sql_types::Text)]
    sku: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    description: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Numeric)]
    quantity: rust_decimal::Decimal,
    #[diesel(sql_type = diesel::sql_types::Text)]
    unit: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
    min_stock: Option<rust_decimal::Decimal>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
    max_stock: Option<rust_decimal::Decimal>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    location: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    category: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Numeric)]
    unit_cost: rust_decimal::Decimal,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    is_active: bool,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    created_at: chrono::DateTime<chrono::Utc>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(diesel::QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct MovementDbRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    branch_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    item_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    movement_type: String,
    #[diesel(sql_type = diesel::sql_types::Numeric)]
    quantity: rust_decimal::Decimal,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    reference_type: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
    reference_id: Option<Uuid>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    notes: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
    created_by: Option<Uuid>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(diesel::QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct PoDbRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    branch_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    po_number: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    vendor_name: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    status: String,
    #[diesel(sql_type = diesel::sql_types::Numeric)]
    total_amount: rust_decimal::Decimal,
    #[diesel(sql_type = diesel::sql_types::Text)]
    currency: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Date>)]
    expected_date: Option<chrono::NaiveDate>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    notes: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    created_at: chrono::DateTime<chrono::Utc>,
}
