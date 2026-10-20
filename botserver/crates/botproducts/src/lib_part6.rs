use super::*;

pub(super) async fn handle_products_items(
    State(state): State<Arc<ProductsState>>,
    headers: HeaderMap,
    Query(query): Query<ProductQuery>,
) -> impl IntoResponse {
    let pool = state.pool.clone();
    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.pool)
        .unwrap_or_else(|| get_bot_context(&state.pool, &state.get_default_bot));

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;

        let cat_map: std::collections::HashMap<String, String> = product_categories::table
            .filter(product_categories::branch_id.eq(branch_id))
            .select((product_categories::id, product_categories::name))
            .load::<(Uuid, String)>(&mut conn)
            .ok()
            .unwrap_or_default()
            .into_iter()
            .map(|(id, name)| (id.to_string(), name))
            .collect();

        let mut db_query = products::table
            .filter(products::branch_id.eq(branch_id))
            .into_boxed();

        if let Some(ref category) = query.category {
            db_query = db_query.filter(products::category.eq(category));
        }

        if let Some(ref status) = query.status {
            let is_active = status == "active";
            db_query = db_query.filter(products::is_active.eq(is_active));
        }

        db_query = db_query.order(products::created_at.desc());

        let limit = query.limit.unwrap_or(50);
        db_query = db_query.limit(limit);

        let items = db_query
            .select((
                products::id,
                products::sku,
                products::name,
                products::description,
                products::category,
                products::price,
                products::currency,
                products::stock_quantity,
                products::is_active,
            ))
            .load::<(Uuid, Option<String>, String, Option<String>, Option<String>, BigDecimal, String, i32, bool)>(&mut conn)
            .ok()?;

        if items.is_empty() {
            return Some(String::new());
        }

        let mut html = String::new();
        for (id, sku, name, desc, category, price, currency, stock, is_active) in items {
            let sku_str = sku.as_deref().filter(|s| !s.is_empty()).unwrap_or("-").to_string();
            let desc_str = desc.unwrap_or_default();
            let cat_id = category.unwrap_or_default();
            let cat_str = cat_map.get(&cat_id).cloned().unwrap_or_else(|| {
                if cat_id.is_empty() { "Uncategorized".to_string() } else { cat_id }
            });
            let price_str = format_currency(bd_to_f64(&price), &currency);
            let stock_str = if stock == -1 { "Unlimited".to_string() } else { stock.to_string() };
            let status_class = if is_active { "status-active" } else { "status-inactive" };
            let status_text = if is_active { "Active" } else { "Inactive" };

            html.push_str(&format!(
                "<div class=\"product-card\" data-id=\"{id}\">\
                <div class=\"product-header\">\
                <span class=\"product-name\">{}</span>\
                <span class=\"product-sku\">{}</span>\
                </div>\
                <div class=\"product-body\">\
                <p class=\"product-desc\">{}</p>\
                <div class=\"product-meta\">\
                <span class=\"product-category\">{}</span>\
                <span class=\"product-price\">{}</span>\
                <span class=\"product-stock\">Stock: {}</span>\
                <span class=\"{}\">{}</span>\
                </div>\
                </div>\
                <div class=\"product-actions\">\
                <button class=\"btn-sm\" hx-get=\"/api/ui/products/items/{id}/detail\" hx-target=\"#products-modal-content\" hx-on::after-request=\"openProductsModal()\">View</button>\
                <button class=\"btn-sm btn-secondary\" hx-get=\"/api/ui/products/items/{id}/edit\" hx-target=\"#products-modal-content\" hx-on::after-request=\"openProductsModal()\">Edit</button>\
                </div>\
                </div>",
                html_escape(&name),
                html_escape(&sku_str),
                html_escape(&desc_str),
                html_escape(&cat_str),
                price_str,
                stock_str,
                status_class,
                status_text
            ));
        }
        Some(html)
    })
    .await
    .ok()
    .flatten();

    match result {
        Some(html) if !html.is_empty() => Html(html),
        _ => Html(
            "<div class=\"products-empty\">\
            <div class=\"empty-icon\">\u{1f4e6}</div>\
            <p>No products yet</p>\
            <p class=\"empty-hint\">Add your first product to get started</p>\
            </div>".to_string(),
        ),
    }
}
pub(super) async fn handle_products_services(
    State(state): State<Arc<ProductsState>>,
    headers: HeaderMap,
    Query(query): Query<ProductQuery>,
) -> impl IntoResponse {
    let pool = state.pool.clone();
    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.pool)
        .unwrap_or_else(|| get_bot_context(&state.pool, &state.get_default_bot));

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;

        let cat_map: std::collections::HashMap<String, String> = product_categories::table
            .filter(product_categories::branch_id.eq(branch_id))
            .select((product_categories::id, product_categories::name))
            .load::<(Uuid, String)>(&mut conn)
            .ok()
            .unwrap_or_default()
            .into_iter()
            .map(|(id, name)| (id.to_string(), name))
            .collect();

        let mut db_query = services::table
            .filter(services::branch_id.eq(branch_id))
            .into_boxed();

        if let Some(ref category) = query.category {
            db_query = db_query.filter(services::category.eq(category));
        }

        if let Some(ref status) = query.status {
            let is_active = status == "active";
            db_query = db_query.filter(services::is_active.eq(is_active));
        }

        db_query = db_query.order(services::created_at.desc());

        let limit = query.limit.unwrap_or(50);
        db_query = db_query.limit(limit);

        let items = db_query
            .select((
                services::id,
                services::name,
                services::description,
                services::category,
                services::service_type,
                services::hourly_rate,
                services::fixed_price,
                services::currency,
                services::duration_minutes,
                services::is_active,
            ))
            .load::<(Uuid, String, Option<String>, Option<String>, String, Option<BigDecimal>, Option<BigDecimal>, String, Option<i32>, bool)>(&mut conn)
            .ok()?;

        if items.is_empty() {
            return Some(String::new());
        }

        let mut html = String::new();
        for (id, name, _desc, category, svc_type, hourly, fixed, currency, duration, is_active) in items {
            let cat_id = category.unwrap_or_default();
            let cat_str = cat_map.get(&cat_id).cloned().unwrap_or_else(|| {
                if cat_id.is_empty() { "General".to_string() } else { cat_id }
            });
            let type_str = svc_type;
            let cur = currency.as_str();
            let price_str = if let Some(ref h) = hourly {
                format!("{}/hr", format_currency(bd_to_f64(h), cur))
            } else if let Some(ref f) = fixed {
                format_currency(bd_to_f64(f), cur)
            } else {
                "-".to_string()
            };
            let duration_str = duration.map(|d| format!("{} min", d)).unwrap_or_else(|| "-".to_string());
            let active = is_active;
            let status_class = if active { "status-active" } else { "status-inactive" };
            let status_text = if active { "Active" } else { "Inactive" };

            html.push_str(&format!(
                "<tr class=\"service-row\" data-id=\"{id}\">\
                <td class=\"service-name\">{}</td>\
                <td class=\"service-category\">{}</td>\
                <td class=\"service-type\">{}</td>\
                <td class=\"service-price\">{}</td>\
                <td class=\"service-duration\">{}</td>\
                <td class=\"service-status\"><span class=\"{}\">{}</span></td>\
                <td class=\"service-actions\">\
                <button class=\"btn-sm\" hx-get=\"/api/products/services/{id}\" hx-target=\"#service-detail\">View</button>\
                </td>\
                </tr>",
                html_escape(&name),
                html_escape(&cat_str),
                html_escape(&type_str),
                price_str,
                duration_str,
                status_class,
                status_text
            ));
        }
        Some(html)
    })
    .await
    .ok()
    .flatten();

    match result {
        Some(html) if !html.is_empty() => Html(html),
        _ => Html(
            "<tr class=\"empty-row\">\
            <td colspan=\"7\" class=\"empty-state\">\
            <div class=\"empty-icon\">\u{1f527}</div>\
            <p>No services yet</p>\
            <p class=\"empty-hint\">Add services to your catalog</p>\
            </td>\
            </tr>".to_string(),
        ),
    }
}
