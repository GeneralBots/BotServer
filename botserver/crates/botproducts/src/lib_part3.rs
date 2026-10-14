use super::*;

pub async fn create_product(
    State(state): State<Arc<ProductsState>>,
    headers: HeaderMap,
    Json(req): Json<CreateProductRequest>,
) -> Result<Json<Product>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts DB error: {e}"); "Internal server error".to_string() })
    })?;

    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.pool)
        .unwrap_or_else(|| get_bot_context(&state.pool, &state.get_default_bot));
    let id = Uuid::new_v4();
    let now = Utc::now();

    let product = Product {
    id,
    branch_id,
    name: req.name,
        sku: req.sku.unwrap_or_default(),
    description: req.description,
    price: Some(bd(req.price)),
    currency: Some(req.currency.unwrap_or_else(|| "USD".to_string())),
    stock_quantity: Some(req.stock_quantity.unwrap_or(0)),
    category_id: None,
    attributes: Some(serde_json::json!({})),
    is_public: None,
    is_active: Some(true),
    created_at: now,
    updated_at: now,
    category: req.category,
    product_type: req.product_type.unwrap_or_else(|| "physical".to_string()),
    cost: req.cost.map(bd),
    tax_rate: bd(req.tax_rate.unwrap_or(0.0)),
    unit: req.unit.unwrap_or_else(|| "unit".to_string()),
    low_stock_threshold: req.low_stock_threshold.unwrap_or(10),
    images: serde_json::json!(req.images.unwrap_or_default()),
    weight: req.weight.map(bd),
    dimensions: None,
    barcode: req.barcode,
};

    diesel::insert_into(products::table)
        .values(&product)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Insert error: {e}"); "Internal server error".to_string() }))?;

    Ok(Json(product))
}
pub async fn list_products(
    State(state): State<Arc<ProductsState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<Product>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts DB error: {e}"); "Internal server error".to_string() })
    })?;

    let has_token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.starts_with("Bearer ") && v.len() > 7)
        .unwrap_or(false);

    // Anonymous storefront callers (no token) may only see the public global
    // catalog (branch nil); authenticated callers get their own tenant-scoped
    // branch products.
    let branch_id = if has_token {
        crate::scope::branch_from_jwt_pool(&headers, &state.pool)
            .unwrap_or_else(|| get_bot_context(&state.pool, &state.get_default_bot))
    } else {
        Uuid::nil()
    };
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let mut q = products::table
        .filter(products::branch_id.eq(branch_id))
        .into_boxed();

    if !has_token {
        q = q.filter(products::is_public.eq(true));
    }

    if let Some(is_active) = query.is_active {
        q = q.filter(products::is_active.eq(is_active));
    }

    if let Some(category) = query.category {
        q = q.filter(products::category.eq(category));
    }

    if let Some(true) = query.low_stock {
        q = q.filter(products::stock_quantity.le(products::low_stock_threshold.nullable()));
    }

    if let Some(search) = query.search {
        let pattern = format!("%{search}%");
        q = q.filter(
            products::name
                .ilike(pattern.clone())
                .or(products::sku.ilike(pattern.clone()))
                .or(products::description.ilike(pattern)),
        );
    }

    let prods: Vec<Product> = q
        .order(products::name.asc())
        .limit(limit)
        .offset(offset)
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Query error: {e}"); "Internal server error".to_string() }))?;

    Ok(Json(prods))
}
pub async fn get_product(
    State(state): State<Arc<ProductsState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ProductWithVariants>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts DB error: {e}"); "Internal server error".to_string() })
    })?;

    // Object-level authorization (issue #734): a product may only be read by
    // a caller whose workspace branch owns it.
    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.pool)
        .ok_or((StatusCode::UNAUTHORIZED, "Authentication required".to_string()))?;

    let product: Product = products::table
        .filter(products::id.eq(id))
        .filter(products::branch_id.eq(branch_id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Product not found".to_string()))?;

    let variants: Vec<ProductVariant> = product_variants::table
        .filter(product_variants::product_id.eq(id))
        .order(product_variants::name.asc())
        .load(&mut conn)
        .unwrap_or_default();

    Ok(Json(ProductWithVariants { product, variants }))
}
pub async fn update_product(
    State(state): State<Arc<ProductsState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProductRequest>,
) -> Result<Json<Product>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts DB error: {e}"); "Internal server error".to_string() })
    })?;

    // Object-level authorization: only the owning branch may modify a product.
    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.pool)
        .ok_or((StatusCode::UNAUTHORIZED, "Authentication required".to_string()))?;
    let now = Utc::now();

    diesel::update(products::table.filter(products::id.eq(id).and(products::branch_id.eq(branch_id))))
        .set(products::updated_at.eq(now))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Update error: {e}"); "Internal server error".to_string() }))?;

    if let Some(name) = req.name {
        diesel::update(products::table.filter(products::id.eq(id).and(products::branch_id.eq(branch_id))))
            .set(products::name.eq(name))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Update error: {e}"); "Internal server error".to_string() }))?;
    }

    if let Some(description) = req.description {
        diesel::update(products::table.filter(products::id.eq(id).and(products::branch_id.eq(branch_id))))
            .set(products::description.eq(description))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Update error: {e}"); "Internal server error".to_string() }))?;
    }

    if let Some(price) = req.price {
        diesel::update(products::table.filter(products::id.eq(id).and(products::branch_id.eq(branch_id))))
            .set(products::price.eq(bd(price)))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Update error: {e}"); "Internal server error".to_string() }))?;
    }

    if let Some(stock_quantity) = req.stock_quantity {
        diesel::update(products::table.filter(products::id.eq(id).and(products::branch_id.eq(branch_id))))
            .set(products::stock_quantity.eq(stock_quantity))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Update error: {e}"); "Internal server error".to_string() }))?;
    }

    if let Some(is_active) = req.is_active {
        diesel::update(products::table.filter(products::id.eq(id).and(products::branch_id.eq(branch_id))))
            .set(products::is_active.eq(is_active))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Update error: {e}"); "Internal server error".to_string() }))?;
    }

    if let Some(category) = req.category {
        diesel::update(products::table.filter(products::id.eq(id).and(products::branch_id.eq(branch_id))))
            .set(products::category.eq(category))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Update error: {e}"); "Internal server error".to_string() }))?;
    }

    let product: Product = products::table
        .filter(products::id.eq(id))
        .filter(products::branch_id.eq(branch_id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Product not found".to_string()))?;

    Ok(Json(product))
}
pub async fn delete_product(
    State(state): State<Arc<ProductsState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts DB error: {e}"); "Internal server error".to_string() })
    })?;

    // Object-level authorization: only the owning branch may delete a product.
    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.pool)
        .ok_or((StatusCode::UNAUTHORIZED, "Authentication required".to_string()))?;

    let deleted = diesel::delete(
        products::table.filter(products::id.eq(id).and(products::branch_id.eq(branch_id))),
    )
    .execute(&mut conn)
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, { log::error!("botproducts Delete error: {e}"); "Internal server error".to_string() }))?;

    if deleted == 0 {
        return Err((StatusCode::NOT_FOUND, "Product not found".to_string()));
    }

    Ok(StatusCode::NO_CONTENT)
}
