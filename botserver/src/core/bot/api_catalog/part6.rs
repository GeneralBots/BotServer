use super::*;

/// Lists CRM/people contacts (read-only), scoped to the bot's branch.
pub(super) async fn list_people(state: &Arc<AppState>, bot_uuid: &Uuid, search: Option<&str>) -> Result<Value, String> {
    let branch = branch_scope(state, bot_uuid)?;
    let term = search.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let mut conn = state.conn.get().map_err(|e| format!("DB error: {e}"))?;
    let rows: Vec<PersonRow> = match &term {
        Some(t) => {
            let pattern = format!("%{t}%");
            diesel::sql_query(
                "SELECT id AS person_id, first_name, last_name, email, status FROM crm_contacts \
                 WHERE branch_id = $1 AND (first_name ILIKE $2 OR last_name ILIKE $2 OR email ILIKE $2) \
                 ORDER BY last_name ASC LIMIT 25",
            )
            .bind::<diesel::sql_types::Uuid, _>(branch)
            .bind::<diesel::sql_types::Text, _>(pattern)
            .load(&mut conn)
            .map_err(|e| format!("Query error: {e}"))?
        }
        None => diesel::sql_query(
            "SELECT id AS person_id, first_name, last_name, email, status FROM crm_contacts \
             WHERE branch_id = $1 ORDER BY last_name ASC LIMIT 25",
        )
        .bind::<diesel::sql_types::Uuid, _>(branch)
        .load(&mut conn)
        .map_err(|e| format!("Query error: {e}"))?,
    };
    let items: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            let name = format!("{} {}", r.first_name.unwrap_or_default(), r.last_name.unwrap_or_default()).trim().to_string();
            json!({
                "person_id": r.person_id,
                "name": if name.is_empty() { "Unnamed".to_string() } else { name },
                "email": r.email.unwrap_or_default(),
                "status": r.status,
                "deep_link": format!("app://people?person_id={}", r.person_id),
            })
        })
        .collect();
    Ok(json!({ "count": items.len(), "people": items }))
}
/// Lists invoices (branch-scoped, read-only).
pub(super) async fn list_invoices(state: &Arc<AppState>, bot_uuid: &Uuid) -> Result<Value, String> {
    let branch = branch_scope(state, bot_uuid)?;
    let mut conn = state.conn.get().map_err(|e| format!("DB error: {e}"))?;
    let rows: Vec<InvoiceRow> = diesel::sql_query(
        "SELECT id AS invoice_id, invoice_number, customer_name, status, total FROM billing_invoices \
         WHERE branch_id = $1 ORDER BY issue_date DESC LIMIT 20",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .map_err(|e| format!("Query error: {e}"))?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|r| json!({
            "invoice_id": r.invoice_id,
            "invoice_number": r.invoice_number,
            "customer_name": r.customer_name,
            "status": r.status,
            "total": r.total.to_string(),
            "deep_link": format!("app://billing?invoice_id={}", r.invoice_id),
        }))
        .collect();
    Ok(json!({ "count": items.len(), "invoices": items }))
}
/// Lists products/services (branch-scoped, read-only).
pub(super) async fn list_products(state: &Arc<AppState>, bot_uuid: &Uuid, category: Option<&str>) -> Result<Value, String> {
    let branch = branch_scope(state, bot_uuid)?;
    let mut conn = state.conn.get().map_err(|e| format!("DB error: {e}"))?;
    let rows: Vec<ProductRow> = match category.map(|s| s.trim()).filter(|s| !s.is_empty()) {
        Some(cat) => diesel::sql_query(
            "SELECT id AS product_id, sku, name, price, product_type, stock_quantity FROM products \
             WHERE branch_id = $1 AND category = $2 ORDER BY name ASC LIMIT 20",
        )
        .bind::<diesel::sql_types::Uuid, _>(branch)
        .bind::<diesel::sql_types::Text, _>(cat)
        .load(&mut conn)
        .map_err(|e| format!("Query error: {e}"))?,
        None => diesel::sql_query(
            "SELECT id AS product_id, sku, name, price, product_type, stock_quantity FROM products \
             WHERE branch_id = $1 ORDER BY name ASC LIMIT 20",
        )
        .bind::<diesel::sql_types::Uuid, _>(branch)
        .load(&mut conn)
        .map_err(|e| format!("Query error: {e}"))?,
    };
    let items: Vec<Value> = rows
        .into_iter()
        .map(|r| json!({
            "product_id": r.product_id,
            "sku": r.sku.unwrap_or_default(),
            "name": r.name,
            "price": r.price.to_string(),
            "product_type": r.product_type,
            "stock_quantity": r.stock_quantity,
            "deep_link": format!("app://products?product_id={}", r.product_id),
        }))
        .collect();
    Ok(json!({ "count": items.len(), "products": items }))
}
/// Lists support tickets (branch-scoped, read-only).
pub(super) async fn list_tickets(state: &Arc<AppState>, bot_uuid: &Uuid) -> Result<Value, String> {
    let branch = branch_scope(state, bot_uuid)?;
    let mut conn = state.conn.get().map_err(|e| format!("DB error: {e}"))?;
    let rows: Vec<TicketRow> = diesel::sql_query(
        "SELECT id AS ticket_id, ticket_number, subject, status, priority FROM support_tickets \
         WHERE branch_id = $1 ORDER BY created_at DESC LIMIT 20",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .map_err(|e| format!("Query error: {e}"))?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|r| json!({
            "ticket_id": r.ticket_id,
            "ticket_number": r.ticket_number,
            "subject": r.subject,
            "status": r.status,
            "priority": r.priority,
            "deep_link": format!("app://tickets?ticket_id={}", r.ticket_id),
        }))
        .collect();
    Ok(json!({ "count": items.len(), "tickets": items }))
}
/// Drive location for the bot a catalog command runs as.
///
/// The bucket cannot be derived from the bot name: an org-hosted bot lives in
/// `{slug}.gborg` with the bot directory as a key prefix, while a standalone
/// bot owns `{bot}.gbai`. Every `drive.*` command goes through this so the
/// chat side addresses exactly the Drive the UI and the file keywords use.
pub(super) fn catalog_drive_location(
    state: &Arc<AppState>,
    bot_uuid: &Uuid,
) -> Result<botbasic_core::utils::BotDriveLocation, String> {
    let mut conn = state.conn.get().map_err(|e| format!("DB error: {e}"))?;
    Ok(botbasic_core::utils::bot_drive_location_for(&mut conn, *bot_uuid))
}
/// Lists drive files under a folder (read-only, via the configured S3 repository).
pub(super) async fn list_drive_files(state: &Arc<AppState>, bot_uuid: &Uuid, path: Option<&str>) -> Result<Value, String> {
    let drive = state.drive.clone().ok_or_else(|| "drive unavailable".to_string())?;
    let loc = catalog_drive_location(state, bot_uuid)?;
    let bucket = loc.bucket.clone();
    let rel = normalize_drive_path(path.unwrap_or(""))?;
    let prefix = if rel.is_empty() { loc.key_prefix.clone() } else { loc.key_for(&rel) };
    let keys = drive
        .list_objects_with_metadata(&bucket, Some(&prefix))
        .await
        .map_err(|e| format!("drive list error: {e}"))?;
    let items: Vec<Value> = keys
        .into_iter()
        .map(|o| json!({ "path": o.key, "size": o.size, "etag": o.etag }))
        .collect();
    Ok(json!({ "count": items.len(), "files": items }))
}
pub(super) async fn execute_tax(
    state: &Arc<AppState>,
    bot_uuid: &Uuid,
    service: Option<String>,
    value: Option<String>,
) -> Result<Value, String> {
    let scope = botbanking::cashflow::resolve_bot_scope(&state.conn, bot_uuid, None)
        .ok_or_else(|| "bot not found".to_string())?;
    let mut conn = state.conn.get().map_err(|e| format!("DB error: {e}"))?;
    let result = match service {
        Some(service) => {
            botproducts::service_tax::calculate_service_tax_inner(
                &mut conn,
                scope.branch_id,
                &service,
                None,
                value.as_deref(),
            )
            .await
        }
        // No registered service named by the user: compute the breakdown for
        // the raw value with the branch tax rates (issue #722).
        None => calculate_anonymous_service_tax(&mut conn, &scope.branch_id, value.as_deref()),
    };
    match result {
        Ok(v) => Ok(v),
        Err((_code, msg)) => Err(msg),
    }
}
/// Computes the tax breakdown for a service value without a registered
/// service (chat asks like "calcular o imposto de um serviço de R$ 10.000").
/// Rates come from `billing_tax_rates` for the branch; the audit row is
/// persisted with a null service id.
pub(super) fn calculate_anonymous_service_tax(
    conn: &mut diesel::PgConnection,
    branch_id: &Uuid,
    value: Option<&str>,
) -> Result<serde_json::Value, (axum::http::StatusCode, String)> {
    use std::str::FromStr as _;
    let service_value = value
        .map(Decimal::from_str)
        .transpose()
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, format!("Invalid value: {e}")))?
        .ok_or_else(|| (axum::http::StatusCode::BAD_REQUEST, "Provide a 'value' for the service".to_string()))?;
    let loaded = bottax::storage::load_rates_from_billing(conn, branch_id);
    let rates = loaded.unwrap_or_default();
    let source = if loaded.is_some() { "billing_tax_rates" } else { "default" };
    let breakdown = bottax::calculator::calculate_service_tax(service_value, &rates);
    if let Err((_code, msg)) = bottax::handlers::persist_calculation(
        conn,
        branch_id,
        None,
        None,
        &breakdown,
        source,
    ) {
        log::warn!("service.tax audit insert skipped: {msg}");
    }
    Ok(serde_json::json!({
        "service": null,
        "service_value": breakdown.service_value.to_string(),
        "breakdown": breakdown,
        "rates": {
            "irpj_pct": rates.irpj_pct.to_string(),
            "csll_pct": rates.csll_pct.to_string(),
            "pis_cofins_pct": rates.pis_cofins_pct.to_string(),
            "iss_pct": rates.iss_pct.to_string(),
        },
        "rate_source": source,
    }))
}
pub(super) async fn write_drive_file(state: &Arc<AppState>, bot_uuid: &Uuid, path: &str, content_b64: &str) -> Result<Value, String> {
    let drive = state
        .drive
        .clone()
        .ok_or_else(|| "drive unavailable".to_string())?;
    let data = base64::engine::general_purpose::STANDARD
        .decode(content_b64)
        .map_err(|e| format!("invalid base64 content: {e}"))?;
    let loc = catalog_drive_location(state, bot_uuid)?;
    let bucket = loc.bucket.clone();
    let key = loc.key_for(&normalize_drive_path(path)?);
    drive
        .put_object(&bucket, &key, data, None)
        .await
        .map_err(|e| e.to_string())?;
    log::info!("api_call drive.write -> {bucket}/{key}");
    Ok(json!({ "success": true, "bucket": bucket, "path": key }))
}
/// Copies a file from one drive path to another (e.g. an attached invoice
/// from `inbox/` into its `faturas/<month>/` folder).
pub(super) async fn move_drive_file(state: &Arc<AppState>, bot_uuid: &Uuid, from: &str, to: &str) -> Result<Value, String> {
    let drive = state
        .drive
        .clone()
        .ok_or_else(|| "drive unavailable".to_string())?;
    let loc = catalog_drive_location(state, bot_uuid)?;
    let bucket = loc.bucket.clone();
    let from_key = loc.key_for(&normalize_drive_path(from)?);
    let to_key = loc.key_for(&normalize_drive_path(to)?);
    let bytes = drive
        .get_object(&bucket, &from_key)
        .await
        .map_err(|e| format!("not found at {from_key}: {e}"))?;
    drive
        .put_object(&bucket, &to_key, bytes, None)
        .await
        .map_err(|e| e.to_string())?;
    log::info!("api_call drive.file -> {bucket}/{to_key}");
    Ok(json!({ "success": true, "bucket": bucket, "from": from_key, "to": to_key }))
}
