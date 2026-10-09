use super::*;

pub(super) async fn handle_product_detail_view(
    State(state): State<Arc<ProductsState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match state.pool.get() {
        Ok(c) => c,
        Err(e) => { log::error!("botproducts DB error: {e}"); return Html(r#"<div class="error">Internal server error</div>"#.to_string()); }
    };

    let product: Product = match products::table
        .filter(products::id.eq(id))
        .first(&mut conn)
    {
        Ok(p) => p,
        Err(_) => return Html("<div class=\"error\">Product not found</div>".to_string()),
    };

    let price_str = format_currency(bd_to_f64(product.price.as_ref().unwrap_or(&BigDecimal::from(0))), product.currency.as_deref().unwrap_or("USD"));
    let cost_str = product.cost.as_ref().map(|c| format_currency(bd_to_f64(c), product.currency.as_deref().unwrap_or("USD"))).unwrap_or_else(|| "-".to_string());
    let stock_str = if product.stock_quantity == Some(-1) { "Unlimited".to_string() } else { product.stock_quantity.map(|v| v.to_string()).unwrap_or_else(|| "-".to_string()) };
    let status_badge = if product.is_active.unwrap_or(false) {
        "<span class=\"status-active\">Active</span>"
    } else {
        "<span class=\"status-inactive\">Inactive</span>"
    };
    let cat_str = product.category.as_deref().unwrap_or("Uncategorized");
    let desc_str = product.description.as_deref().unwrap_or("No description");
    let sku_str = if product.sku.is_empty() { "-" } else { &product.sku };
    let tax_rate = bd_to_f64(&product.tax_rate);
    let created = product.created_at.format("%Y-%m-%d %H:%M UTC");
    let updated = product.updated_at.format("%Y-%m-%d %H:%M UTC");

    Html(format!(
        "<div class=\"product-detail\">\
        <div class=\"product-form-header\">\
        <h2 class=\"product-form-title\">{name}</h2>\
        <button type=\"button\" class=\"form-close\" onclick=\"closeProductsModal()\">\
        <svg width=\"20\" height=\"20\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\">\
        <line x1=\"18\" y1=\"6\" x2=\"6\" y2=\"18\"></line>\
        <line x1=\"6\" y1=\"6\" x2=\"18\" y2=\"18\"></line>\
        </svg></button></div>\
        <div class=\"detail-grid\">\
        <div class=\"detail-field\"><label>SKU</label><span>{sku}</span></div>\
        <div class=\"detail-field\"><label>Category</label><span>{cat}</span></div>\
        <div class=\"detail-field\"><label>Price</label><span class=\"product-price\">{price}</span></div>\
        <div class=\"detail-field\"><label>Cost</label><span>{cost}</span></div>\
        <div class=\"detail-field\"><label>Tax Rate</label><span>{tax_rate}%</span></div>\
        <div class=\"detail-field\"><label>Stock</label><span>{stock}</span></div>\
        <div class=\"detail-field\"><label>Unit</label><span>{unit}</span></div>\
        <div class=\"detail-field\"><label>Status</label><span>{status}</span></div>\
        <div class=\"detail-field\"><label>Barcode</label><span>{barcode}</span></div>\
        <div class=\"detail-field\"><label>Weight</label><span>{weight}</span></div>\
        </div>\
        <div class=\"detail-section\"><label>Description</label><p>{desc}</p></div>\
        <div class=\"detail-section detail-meta\"><small>Created: {created} &mdash; Updated: {updated}</small></div>\
        </div>",
        name = html_escape(&product.name),
        sku = html_escape(sku_str),
        cat = html_escape(cat_str),
        price = price_str,
        cost = cost_str,
        tax_rate = tax_rate,
        stock = stock_str,
        unit = html_escape(&product.unit),
        status = status_badge,
        barcode = product.barcode.as_deref().unwrap_or("-"),
        weight = product.weight.as_ref().map(|w| format!("{} kg", bd_to_f64(w))).unwrap_or_else(|| "-".to_string()),
        desc = html_escape(desc_str),
        created = created,
        updated = updated,
    ))
}
pub(super) async fn handle_product_edit_form(
    State(state): State<Arc<ProductsState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let mut conn = match state.pool.get() {
        Ok(c) => c,
        Err(e) => { log::error!("botproducts DB error: {e}"); return Html(r#"<div class="error">Internal server error</div>"#.to_string()); }
    };

    let product: Product = match products::table
        .filter(products::id.eq(id))
        .first(&mut conn)
    {
        Ok(p) => p,
        Err(_) => return Html("<div class=\"error\">Product not found</div>".to_string()),
    };

    let price_str = product.price.as_ref().map(|p| p.to_string()).unwrap_or_else(|| "0".to_string());
    let cost_str = product.cost.as_ref().map(|c| c.to_string()).unwrap_or_default();
    let tax_rate_str = product.tax_rate.to_string();
    let active_sel = |v: bool, expected: bool| {
        if v == expected { "selected" } else { "" }
    };
    let cat_sel = |c: &str| {
        if product.category.as_deref() == Some(c) { "selected" } else { "" }
    };
    let unit_sel = |u: &str| {
        if product.unit == u { "selected" } else { "" }
    };

    Html(format!(
        "<form class=\"product-form\" hx-put=\"/api/products/items/{id}\" hx-target=\"#products-grid\" hx-swap=\"innerHTML\" hx-on::after-request=\"closeProductsModal()\">\
        <div class=\"product-form-header\">\
        <h2 class=\"product-form-title\">Edit Product</h2>\
        <button type=\"button\" class=\"form-close\" onclick=\"closeProductsModal()\">\
        <svg width=\"20\" height=\"20\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\">\
        <line x1=\"18\" y1=\"6\" x2=\"6\" y2=\"18\"></line>\
        <line x1=\"6\" y1=\"6\" x2=\"18\" y2=\"18\"></line>\
        </svg></button></div>\
        <div class=\"form-row\">\
        <div class=\"form-group\"><label class=\"form-label\">Product Name *</label>\
        <input type=\"text\" name=\"name\" class=\"form-input\" value=\"{name}\" required></div>\
        <div class=\"form-group\"><label class=\"form-label\">SKU</label>\
        <input type=\"text\" name=\"sku\" class=\"form-input\" value=\"{sku}\"></div>\
        </div>\
        <div class=\"form-group\"><label class=\"form-label\">Description</label>\
        <textarea name=\"description\" class=\"form-textarea\" rows=\"3\">{desc}</textarea></div>\
        <div class=\"form-row\">\
        <div class=\"form-group\"><label class=\"form-label\">Price *</label>\
        <input type=\"number\" name=\"price\" class=\"form-input\" value=\"{price}\" required min=\"0\" step=\"0.01\"></div>\
        <div class=\"form-group\"><label class=\"form-label\">Cost</label>\
        <input type=\"number\" name=\"cost\" class=\"form-input\" value=\"{cost}\" min=\"0\" step=\"0.01\"></div>\
        </div>\
        <div class=\"form-row\">\
        <div class=\"form-group\"><label class=\"form-label\">Category *</label>\
        <select name=\"category\" class=\"form-select\" required>\
        <option value=\"\">Select category...</option>\
        <option value=\"software\" {cat_sel_sw}>Software</option>\
        <option value=\"hardware\" {cat_sel_hw}>Hardware</option>\
        <option value=\"subscription\" {cat_sel_sub}>Subscription</option>\
        <option value=\"consulting\" {cat_sel_con}>Consulting</option>\
        <option value=\"training\" {cat_sel_tr}>Training</option>\
        <option value=\"support\" {cat_sel_sup}>Support</option>\
        <option value=\"other\" {cat_sel_oth}>Other</option>\
        </select></div>\
        <div class=\"form-group\"><label class=\"form-label\">Unit</label>\
        <select name=\"unit\" class=\"form-select\">\
        <option value=\"unit\" {unit_u}>Unit</option>\
        <option value=\"license\" {unit_l}>License</option>\
        <option value=\"seat\" {unit_s}>Seat</option>\
        <option value=\"hour\" {unit_h}>Hour</option>\
        <option value=\"month\" {unit_m}>Month</option>\
        <option value=\"year\" {unit_y}>Year</option>\
        </select></div>\
        </div>\
        <div class=\"form-row\">\
        <div class=\"form-group\"><label class=\"form-label\">Status</label>\
        <select name=\"status\" class=\"form-select\">\
        <option value=\"active\" {status_a}>Active</option>\
        <option value=\"inactive\" {status_i}>Inactive</option>\
        </select></div>\
        <div class=\"form-group\"><label class=\"form-label\">Tax Rate (%)</label>\
        <input type=\"number\" name=\"tax_rate\" class=\"form-input\" value=\"{tax_rate}\" min=\"0\" step=\"0.01\"></div>\
        </div>\
        <div class=\"form-row\">\
        <div class=\"form-group\"><label class=\"form-label\">Stock Quantity</label>\
        <input type=\"number\" name=\"stock_quantity\" class=\"form-input\" value=\"{stock}\" min=\"-1\"></div>\
        <div class=\"form-group\"><label class=\"form-label\">Barcode</label>\
        <input type=\"text\" name=\"barcode\" class=\"form-input\" value=\"{barcode}\"></div>\
        </div>\
        <div class=\"form-actions\">\
        <button type=\"button\" class=\"form-btn secondary\" onclick=\"closeProductsModal()\">Cancel</button>\
        <button type=\"submit\" class=\"form-btn primary\">Update Product</button>\
        </div>\
        </form>",
        id = id,
        name = html_escape(&product.name),
        sku = html_escape(&product.sku),
        desc = html_escape(&product.description.as_deref().unwrap_or("")),
        price = price_str,
        cost = cost_str,
        cat_sel_sw = cat_sel("software"),
        cat_sel_hw = cat_sel("hardware"),
        cat_sel_sub = cat_sel("subscription"),
        cat_sel_con = cat_sel("consulting"),
        cat_sel_tr = cat_sel("training"),
        cat_sel_sup = cat_sel("support"),
        cat_sel_oth = cat_sel("other"),
        unit_u = unit_sel("unit"),
        unit_l = unit_sel("license"),
        unit_s = unit_sel("seat"),
        unit_h = unit_sel("hour"),
        unit_m = unit_sel("month"),
        unit_y = unit_sel("year"),
        status_a = active_sel(product.is_active.unwrap_or(false), true),
        status_i = active_sel(product.is_active.unwrap_or(false), false),
        tax_rate = tax_rate_str,
        stock = product.stock_quantity.map(|v| v.to_string()).unwrap_or_else(|| "0".to_string()),
        barcode = html_escape(&product.barcode.as_deref().unwrap_or("")),
    ))
}
pub(super) async fn handle_products_debug() -> &'static str {
    "products debug ok"
}
pub fn configure_products_routes() -> Router<Arc<ProductsState>> {
    Router::new()
        .route("/api/ui/products/items", get(handle_products_items))
        .route("/api/ui/products/items/:id/detail", get(handle_product_detail_view))
        .route("/api/ui/products/items/:id/edit", get(handle_product_edit_form))
        .route("/api/ui/products/items/:id/debug", get(handle_products_debug))
        .route("/api/ui/products/services", get(handle_products_services))
        .route("/api/ui/products/pricelists", get(handle_products_pricelists))
        .route("/api/ui/products/stats/total-products", get(handle_total_products))
        .route("/api/ui/products/stats/total-services", get(handle_total_services))
        .route("/api/ui/products/stats/pricelists", get(handle_total_pricelists))
        .route("/api/ui/products/stats/active", get(handle_active_products))
        .route("/api/ui/products/search", get(handle_products_search))
}
