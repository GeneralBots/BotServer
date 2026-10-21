use super::*;

pub async fn adjust_stock(
    State(state): State<Arc<ProductsState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<AdjustStockRequest>,
) -> Result<Json<Product>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts DB error: {e}"); "Internal server error".to_string() })
    })?;

    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.pool)
        .unwrap_or_else(|| get_bot_context(&state.pool, &state.get_default_bot));
    let now = Utc::now();

    let product: Product = products::table
        .filter(products::id.eq(id))
        .filter(products::branch_id.eq(branch_id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Product not found".to_string()))?;

    let current_stock = product.stock_quantity;
    let new_quantity = match req.movement_type.as_str() {
        "in" | "purchase" | "return" | "adjustment_add" => current_stock + req.quantity,
        "out" | "sale" | "adjustment_remove" | "damage" => current_stock - req.quantity,
        "set" => req.quantity,
        _ => current_stock + req.quantity,
    };

    diesel::update(products::table.filter(products::id.eq(id).and(products::branch_id.eq(branch_id))))
        .set((
            products::stock_quantity.eq(new_quantity),
            products::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Update error: {e}"); "Internal server error".to_string() }))?;

    let movement = InventoryMovement {
        id: Uuid::new_v4(),
        org_id: org_for_branch(&state.pool, branch_id),
        bot_id: bot_for_branch(&state.pool, branch_id),
        product_id: id,
        movement_type: req.movement_type,
        quantity: req.quantity,
        reference_type: req.reference_type,
        reference_id: req.reference_id,
        notes: req.notes,
        created_by: None,
        created_at: now,
        branch_id,
    };

    diesel::insert_into(inventory_movements::table)
        .values(&movement)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Insert error: {e}"); "Internal server error".to_string() }))?;

    let updated: Product = products::table
        .filter(products::id.eq(id))
        .filter(products::branch_id.eq(branch_id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Product not found".to_string()))?;

    Ok(Json(updated))
}
pub async fn create_service(
    State(state): State<Arc<ProductsState>>,
    headers: HeaderMap,
    Json(req): Json<CreateServiceRequest>,
) -> Result<Json<Service>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts DB error: {e}"); "Internal server error".to_string() })
    })?;

    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.pool)
        .unwrap_or_else(|| get_bot_context(&state.pool, &state.get_default_bot));
    let id = Uuid::new_v4();
    let now = Utc::now();

    let service = Service {
        id,
        // `services.org_id`/`bot_id` are NOT NULL since `6.0.10-01-products`;
        // the branch they belong to owns both.
        org_id: org_for_branch(&state.pool, branch_id),
        bot_id: bot_for_branch(&state.pool, branch_id),
        branch_id,
        name: req.name,
        description: req.description,
        category: req.category,
        service_type: req.service_type.unwrap_or_else(|| "hourly".to_string()),
        hourly_rate: req.hourly_rate.map(bd),
        fixed_price: req.fixed_price.map(bd),
        currency: req.currency.unwrap_or_else(|| "USD".to_string()),
        duration_minutes: req.duration_minutes,
        is_active: true,
        attributes: serde_json::json!({}),
        created_at: now,
        updated_at: now,
        tax_rate: bd(0.0),
    };

    diesel::insert_into(services::table)
        .values(&service)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Insert error: {e}"); "Internal server error".to_string() }))?;

    Ok(Json(service))
}
pub async fn list_services(
    State(state): State<Arc<ProductsState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<Service>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts DB error: {e}"); "Internal server error".to_string() })
    })?;

    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.pool)
        .unwrap_or_else(|| get_bot_context(&state.pool, &state.get_default_bot));
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let mut q = services::table
        .filter(services::branch_id.eq(branch_id))
        
        .into_boxed();

    if let Some(is_active) = query.is_active {
        q = q.filter(services::is_active.eq(is_active));
    }

    if let Some(category) = query.category {
        q = q.filter(services::category.eq(category));
    }

    if let Some(search) = query.search {
        let pattern = format!("%{search}%");
        q = q.filter(
            services::name
                .ilike(pattern.clone())
                .or(services::description.ilike(pattern)),
        );
    }

    let svcs: Vec<Service> = q
        .order(services::name.asc())
        .limit(limit)
        .offset(offset)
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Query error: {e}"); "Internal server error".to_string() }))?;

    Ok(Json(svcs))
}
pub async fn get_service(
    State(state): State<Arc<ProductsState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Service>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts DB error: {e}"); "Internal server error".to_string() })
    })?;

    let service: Service = services::table
        .filter(services::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Service not found".to_string()))?;

    Ok(Json(service))
}
pub async fn update_service(
    State(state): State<Arc<ProductsState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateServiceRequest>,
) -> Result<Json<Service>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts DB error: {e}"); "Internal server error".to_string() })
    })?;

    let now = Utc::now();

    diesel::update(services::table.filter(services::id.eq(id)))
        .set(services::updated_at.eq(now))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Update error: {e}"); "Internal server error".to_string() }))?;

    if let Some(name) = req.name {
        diesel::update(services::table.filter(services::id.eq(id)))
            .set(services::name.eq(name))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Update error: {e}"); "Internal server error".to_string() }))?;
    }

    if let Some(description) = req.description {
        diesel::update(services::table.filter(services::id.eq(id)))
            .set(services::description.eq(description))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Update error: {e}"); "Internal server error".to_string() }))?;
    }

    if let Some(hourly_rate) = req.hourly_rate {
        diesel::update(services::table.filter(services::id.eq(id)))
            .set(services::hourly_rate.eq(Some(bd(hourly_rate))))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Update error: {e}"); "Internal server error".to_string() }))?;
    }

    if let Some(is_active) = req.is_active {
        diesel::update(services::table.filter(services::id.eq(id)))
            .set(services::is_active.eq(is_active))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Update error: {e}"); "Internal server error".to_string() }))?;
    }

    let service: Service = services::table
        .filter(services::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Service not found".to_string()))?;

    Ok(Json(service))
}
pub async fn delete_service(
    State(state): State<Arc<ProductsState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts DB error: {e}"); "Internal server error".to_string() })
    })?;

    diesel::delete(services::table.filter(services::id.eq(id)))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Delete error: {e}"); "Internal server error".to_string() }))?;

    Ok(StatusCode::NO_CONTENT)
}
pub async fn list_categories(
    State(state): State<Arc<ProductsState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ProductCategory>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts DB error: {e}"); "Internal server error".to_string() })
    })?;

    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.pool)
        .unwrap_or_else(|| get_bot_context(&state.pool, &state.get_default_bot));

    let cats: Vec<ProductCategory> = product_categories::table
        .filter(product_categories::branch_id.eq(branch_id))
        
        .filter(product_categories::is_active.eq(true))
        .order(product_categories::sort_order.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Query error: {e}"); "Internal server error".to_string() }))?;

    Ok(Json(cats))
}
pub async fn create_category(
    State(state): State<Arc<ProductsState>>,
    headers: HeaderMap,
    Json(req): Json<CreateCategoryRequest>,
) -> Result<Json<ProductCategory>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts DB error: {e}"); "Internal server error".to_string() })
    })?;

    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.pool)
        .unwrap_or_else(|| get_bot_context(&state.pool, &state.get_default_bot));
    let id = Uuid::new_v4();
    let now = Utc::now();

    let max_order: Option<i32> = product_categories::table
        .filter(product_categories::branch_id.eq(branch_id))
        
        .select(diesel::dsl::max(product_categories::sort_order))
        .first(&mut conn)
        .unwrap_or(None);

    let category = ProductCategory {
        id,
        // Nullable by design: `6.5.15.1`/`6.5.98` drop NOT NULL on both so the
        // catalog seed can insert categories without a resolved bot context.
        org_id: None,
        bot_id: None,
        branch_id,
        name: req.name,
        description: req.description,
        parent_id: req.parent_id,
        slug: req.slug,
        image_url: req.image_url,
        sort_order: max_order.unwrap_or(0) + 1,
        is_active: true,
        created_at: now,
        display_order: None,
        updated_at: now,
    };

    diesel::insert_into(product_categories::table)
        .values(&category)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Insert error: {e}"); "Internal server error".to_string() }))?;

    Ok(Json(category))
}
