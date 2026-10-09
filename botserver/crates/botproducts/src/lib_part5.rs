use super::*;

pub async fn list_price_lists(
    State(state): State<Arc<ProductsState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<PriceList>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts DB error: {e}"); "Internal server error".to_string() })
    })?;

    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.pool)
        .unwrap_or_else(|| get_bot_context(&state.pool, &state.get_default_bot));

    let lists: Vec<PriceList> = price_lists::table
        .filter(price_lists::branch_id.eq(branch_id))
        
        .filter(price_lists::is_active.eq(true))
        .order(price_lists::name.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Query error: {e}"); "Internal server error".to_string() }))?;

    Ok(Json(lists))
}
pub async fn create_price_list(
    State(state): State<Arc<ProductsState>>,
    headers: HeaderMap,
    Json(req): Json<CreatePriceListRequest>,
) -> Result<Json<PriceList>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts DB error: {e}"); "Internal server error".to_string() })
    })?;

    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.pool)
        .unwrap_or_else(|| get_bot_context(&state.pool, &state.get_default_bot));
    let id = Uuid::new_v4();
    let now = Utc::now();

    let valid_from = req
        .valid_from
        .and_then(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok());

    let valid_until = req
        .valid_until
        .and_then(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok());

    let price_list = PriceList {
    id: id,
    branch_id: branch_id,
    name: req.name,
    currency: Some(req.currency.unwrap_or_else(|| "USD".to_string())),
    is_active: Some(true),
    valid_from: valid_from,
    valid_until: valid_until,
    created_at: now,
    updated_at: now,
    description: req.description,
    is_default: false,
    customer_group: req.customer_group,
    discount_percent: bd(req.discount_percent.unwrap_or(0.0)),
};

    diesel::insert_into(price_lists::table)
        .values(&price_list)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Insert error: {e}"); "Internal server error".to_string() }))?;

    Ok(Json(price_list))
}
pub async fn list_inventory_movements(
    State(state): State<Arc<ProductsState>>,
    Path(product_id): Path<Uuid>,
) -> Result<Json<Vec<InventoryMovement>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts DB error: {e}"); "Internal server error".to_string() })
    })?;

    let movements: Vec<InventoryMovement> = inventory_movements::table
        .filter(inventory_movements::product_id.eq(product_id))
        .order(inventory_movements::created_at.desc())
        .limit(100)
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Query error: {e}"); "Internal server error".to_string() }))?;

    Ok(Json(movements))
}
pub async fn get_product_stats(
    State(state): State<Arc<ProductsState>>,
    headers: HeaderMap,
) -> Result<Json<ProductStats>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts DB error: {e}"); "Internal server error".to_string() })
    })?;

    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.pool)
        .unwrap_or_else(|| get_bot_context(&state.pool, &state.get_default_bot));

    let total_products: i64 = products::table
        .filter(products::branch_id.eq(branch_id))
        
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let active_products: i64 = products::table
        .filter(products::branch_id.eq(branch_id))
        
        .filter(products::is_active.eq(true))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let total_services: i64 = services::table
        .filter(services::branch_id.eq(branch_id))
        
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let active_services: i64 = services::table
        .filter(services::branch_id.eq(branch_id))
        
        .filter(services::is_active.eq(true))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let low_stock_count: i64 = products::table
        .filter(products::branch_id.eq(branch_id))
        
        .filter(products::is_active.eq(true))
        .filter(products::stock_quantity.le(products::low_stock_threshold.nullable()))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let categories_count: i64 = product_categories::table
        .filter(product_categories::branch_id.eq(branch_id))
        
        .filter(product_categories::is_active.eq(true))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let price_lists_count: i64 = price_lists::table
        .filter(price_lists::branch_id.eq(branch_id))
        
        .filter(price_lists::is_active.eq(true))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let all_products: Vec<Product> = products::table
        .filter(products::branch_id.eq(branch_id))
        
        .filter(products::is_active.eq(true))
        .load(&mut conn)
        .unwrap_or_default();

    let total_stock_value: f64 = all_products
        .iter()
        .map(|p| bd_to_f64(p.price.as_ref().unwrap_or(&BigDecimal::from(0))) * p.stock_quantity.unwrap_or(0) as f64)
        .sum();

    let stats = ProductStats {
        total_products,
        active_products,
        total_services,
        active_services,
        low_stock_count,
        total_stock_value,
        categories_count,
        price_lists_count,
    };

    Ok(Json(stats))
}
pub async fn list_low_stock(
    State(state): State<Arc<ProductsState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<Product>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts DB error: {e}"); "Internal server error".to_string() })
    })?;

    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.pool)
        .unwrap_or_else(|| get_bot_context(&state.pool, &state.get_default_bot));

    let prods: Vec<Product> = products::table
        .filter(products::branch_id.eq(branch_id))
        
        .filter(products::is_active.eq(true))
        .filter(products::stock_quantity.le(products::low_stock_threshold.nullable()))
        .order(products::stock_quantity.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Query error: {e}"); "Internal server error".to_string() }))?;

    Ok(Json(prods))
}
pub fn configure_products_api_routes() -> Router<Arc<ProductsState>> {
    Router::new()
        .route("/api/products/items", get(list_products).post(create_product))
        .route("/api/products/items/:id", get(get_product).put(update_product).delete(delete_product))
        .route("/api/products/items/:id/stock", put(adjust_stock))
        .route("/api/products/items/:id/movements", get(list_inventory_movements))
        .route("/api/products/services", get(list_services).post(create_service))
        .route("/api/products/services/:id", get(get_service).put(update_service).delete(delete_service))
        .route("/api/products/services/:id/tax", post(service_tax::calculate_service_tax))
        .route("/api/products/categories", get(list_categories).post(create_category))
        .route("/api/products/price-lists", get(list_price_lists).post(create_price_list))
        .route("/api/products/stats", get(get_product_stats))
        .route("/api/products/low-stock", get(list_low_stock))
}
