use super::*;

pub(super) async fn handle_products_pricelists(
    State(state): State<Arc<ProductsState>>,
    headers: HeaderMap,
    Query(query): Query<ProductQuery>,
) -> impl IntoResponse {
    let pool = state.pool.clone();
    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.pool)
        .unwrap_or_else(|| get_bot_context(&state.pool, &state.get_default_bot));

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;
        let mut db_query = price_lists::table
            .filter(price_lists::branch_id.eq(branch_id))
            .into_boxed();

        if let Some(ref status) = query.status {
            let is_active = status == "active";
            db_query = db_query.filter(price_lists::is_active.eq(is_active));
        }

        db_query = db_query.order(price_lists::created_at.desc());

        let limit = query.limit.unwrap_or(50);
        db_query = db_query.limit(limit);

        db_query
            .select((
                price_lists::id,
                price_lists::name,
                price_lists::description,
                price_lists::currency,
                price_lists::is_default,
                price_lists::discount_percent,
                price_lists::customer_group,
                price_lists::is_active,
            ))
            .load::<(Uuid, String, Option<String>, Option<String>, bool, BigDecimal, Option<String>, Option<bool>)>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    match result {
        Some(items) if !items.is_empty() => {
            let mut html = String::new();
            for (id, name, _desc, currency, is_default, discount, customer_group, is_active) in items {
                let currency = currency.unwrap_or_default();
                let is_active = is_active.unwrap_or(true);
                let discount_pct = bd_to_f64(&discount);
                let discount_str = if discount_pct > 0.0 { format!("{:.1}%", discount_pct) } else { "-".to_string() };
                let group_str = customer_group.unwrap_or_else(|| "All".to_string());
                let default_badge = if is_default { "<span class=\"badge badge-primary\">Default</span>" } else { "" };
                let status_class = if is_active { "status-active" } else { "status-inactive" };
                let status_text = if is_active { "Active" } else { "Inactive" };

                html.push_str(&format!(
                    "<tr class=\"pricelist-row\" data-id=\"{id}\">\
                    <td class=\"pricelist-name\">{} {}</td>\
                    <td class=\"pricelist-currency\">{}</td>\
                    <td class=\"pricelist-discount\">{}</td>\
                    <td class=\"pricelist-group\">{}</td>\
                    <td class=\"pricelist-status\"><span class=\"{}\">{}</span></td>\
                    <td class=\"pricelist-actions\">\
                    <button class=\"btn-sm\" hx-get=\"/api/products/pricelists/{id}\" hx-target=\"#pricelist-detail\">View</button>\
                    </td>\
                    </tr>",
                    html_escape(&name),
                    default_badge,
                    currency,
                    discount_str,
                    html_escape(&group_str),
                    status_class,
                    status_text
                ));
            }
            Html(html)
        }
        _ => Html(
            "<tr class=\"empty-row\">\
            <td colspan=\"6\" class=\"empty-state\">\
            <div class=\"empty-icon\">\u{1f4b0}</div>\
            <p>No price lists yet</p>\
            <p class=\"empty-hint\">Create price lists for different customer segments</p>\
            </td>\
            </tr>".to_string(),
        ),
    }
}
pub(super) async fn handle_total_products(
    State(state): State<Arc<ProductsState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let pool = state.pool.clone();
    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.pool)
        .unwrap_or_else(|| get_bot_context(&state.pool, &state.get_default_bot));

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;

        products::table
            .filter(products::branch_id.eq(branch_id))
            .count()
            .get_result::<i64>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    Html(format!("{}", result.unwrap_or(0)))
}
pub(super) async fn handle_total_services(
    State(state): State<Arc<ProductsState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let pool = state.pool.clone();
    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.pool)
        .unwrap_or_else(|| get_bot_context(&state.pool, &state.get_default_bot));

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;

        services::table
            .filter(services::branch_id.eq(branch_id))
            .count()
            .get_result::<i64>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    Html(format!("{}", result.unwrap_or(0)))
}
pub(super) async fn handle_total_pricelists(
    State(state): State<Arc<ProductsState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let pool = state.pool.clone();
    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.pool)
        .unwrap_or_else(|| get_bot_context(&state.pool, &state.get_default_bot));

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;

        price_lists::table
            .filter(price_lists::branch_id.eq(branch_id))
            .count()
            .get_result::<i64>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    Html(format!("{}", result.unwrap_or(0)))
}
pub(super) async fn handle_active_products(
    State(state): State<Arc<ProductsState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let pool = state.pool.clone();
    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.pool)
        .unwrap_or_else(|| get_bot_context(&state.pool, &state.get_default_bot));

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;

        products::table
            .filter(products::branch_id.eq(branch_id))
            .filter(products::is_active.eq(true))
            .count()
            .get_result::<i64>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    Html(format!("{}", result.unwrap_or(0)))
}
pub(super) async fn handle_products_search(
    State(state): State<Arc<ProductsState>>,
    headers: HeaderMap,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let q = query.q.clone().unwrap_or_default();
    if q.is_empty() {
        return Html(String::new());
    }

    let pool = state.pool.clone();
    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.pool)
        .unwrap_or_else(|| get_bot_context(&state.pool, &state.get_default_bot));
    let search_term = format!("%{}%", q);

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

        let items = products::table
            .filter(products::branch_id.eq(branch_id))
            .filter(
                products::name.ilike(&search_term)
                    .or(products::sku.ilike(&search_term))
                    .or(products::description.ilike(&search_term))
            )
            .order(products::name.asc())
            .limit(20)
            .select((
                products::id,
                products::sku,
                products::name,
                products::category,
                products::price,
                products::currency,
            ))
            .load::<(Uuid, String, String, Option<String>, Option<BigDecimal>, Option<String>)>(&mut conn)
            .ok()?;

        if items.is_empty() {
            return Some(String::new());
        }

        let mut html = String::new();
        for (id, sku, name, category, price, currency) in items {
            let price = price.unwrap_or_default();
            let currency = currency.unwrap_or_default();
            let sku_str = if sku.is_empty() { "-".to_string() } else { sku };
            let cat_id = category.unwrap_or_default();
            let cat_str = cat_map.get(&cat_id).cloned().unwrap_or_else(|| {
                if cat_id.is_empty() { "Uncategorized".to_string() } else { cat_id }
            });
            let price_str = format_currency(bd_to_f64(&price), &currency);

            html.push_str(&format!(
                "<div class=\"search-result-item\" hx-get=\"/api/ui/products/items/{id}/detail\" hx-target=\"#products-modal-content\" hx-on::after-request=\"openProductsModal()\">\
                <span class=\"result-name\">{}</span>\
                <span class=\"result-sku\">{}</span>\
                <span class=\"result-category\">{}</span>\
                <span class=\"result-price\">{}</span>\
                </div>",
                html_escape(&name),
                html_escape(&sku_str),
                html_escape(&cat_str),
                price_str
            ));
        }
        Some(format!("<div class=\"search-results\">{html}</div>"))
    })
    .await
    .ok()
    .flatten();

    match result {
        Some(html) if !html.is_empty() => Html(html),
        _ => Html(format!(
            "<div class=\"search-results-empty\">\
            <p>No results for \"{}\"</p>\
            </div>",
            html_escape(&q)
        )),
    }
}
