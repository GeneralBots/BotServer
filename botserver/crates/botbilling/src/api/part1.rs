use super::*;

pub async fn create_invoice(
    State(state): State<Arc<BillingApiState>>,
    Json(req): Json<CreateInvoiceRequest>,
) -> Result<Json<BillingInvoice>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = get_bot_context(&state.pool, &state.get_default_bot);
    let (org_id, bot_id) = scope_for_branch(&mut conn, branch_id);
    let id = Uuid::new_v4();
    let now = Utc::now();
    let invoice_number = generate_invoice_number(&mut conn, branch_id);

    let issue_date = req
        .issue_date
        .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
        .unwrap_or_else(|| now.date_naive());

    let due_date = req
        .due_date
        .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
        .unwrap_or_else(|| issue_date + chrono::Duration::days(30));

    let tax_rate = bd(req.tax_rate.unwrap_or(0.0));
    let discount_percent = bd(req.discount_percent.unwrap_or(0.0));

    let mut subtotal = bd(0.0);
    let items = req.items.unwrap_or_default();
    for item in &items {
        let item_amount = item.quantity * item.unit_price;
        let item_discount = item_amount * item.discount_percent.unwrap_or(0.0) / 100.0;
        subtotal += bd(item_amount - item_discount);
    }

    let discount_amount = &subtotal * &discount_percent / bd(100.0);
    let taxable = &subtotal - &discount_amount;
    let tax_amount = &taxable * &tax_rate / bd(100.0);
    let total = &taxable + &tax_amount;

    let invoice = BillingInvoice {
        id,
        org_id,
        bot_id,
        branch_id,
        invoice_number,
        customer_name: req.customer_name.clone().unwrap_or_default(),
        customer_email: req.customer_email.clone(),
        status: "draft".to_string(),
        total: total.clone(),
        currency: req.currency.clone().unwrap_or_else(|| "USD".to_string()),
        due_date,
        paid_at: None,
        notes: req.notes.clone(),
        created_at: now,
        updated_at: now,
        customer_id: req.customer_id,
        customer_address: req.customer_address.clone(),
        issue_date,
        subtotal: subtotal.clone(),
        tax_rate,
        tax_amount,
        discount_percent,
        discount_amount,
        amount_paid: bd(0.0),
        amount_due: total,
        terms: req.terms.clone(),
        footer: None,
        sent_at: None,
        voided_at: None,
    };

    diesel::insert_into(billing_invoices::table)
        .values(&invoice)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    for (idx, item) in items.iter().enumerate() {
        let item_amount = item.quantity * item.unit_price;
        let item_discount = item_amount * item.discount_percent.unwrap_or(0.0) / 100.0;
        let final_amount = item_amount - item_discount;

        let inv_item = BillingInvoiceItem {
            id: Uuid::new_v4(),
            invoice_id: id,
            product_id: item.product_id,
            description: item.description.clone(),
            quantity: bd(item.quantity),
            unit_price: bd(item.unit_price),
            discount_percent: bd(item.discount_percent.unwrap_or(0.0)),
            tax_rate: bd(item.tax_rate.unwrap_or(0.0)),
            amount: bd(final_amount),
            sort_order: idx as i32,
            created_at: now,
        };

        diesel::insert_into(billing_invoice_items::table)
            .values(&inv_item)
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert item error: {e}")))?;
    }

    Ok(Json(invoice))
}
pub async fn list_invoices(
    State(state): State<Arc<BillingApiState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<BillingInvoice>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = get_bot_context(&state.pool, &state.get_default_bot);
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let mut q = billing_invoices::table
        .filter(billing_invoices::branch_id.eq(branch_id))
        .into_boxed();

    if let Some(status) = query.status {
        if status != "all" {
            q = q.filter(billing_invoices::status.eq(status));
        }
    }

    if let Some(customer_id) = query.customer_id {
        q = q.filter(billing_invoices::customer_id.eq(customer_id));
    }

    if let Some(search) = query.search {
        let pattern = format!("%{search}%");
        q = q.filter(
            billing_invoices::invoice_number
                .ilike(pattern.clone())
                .or(billing_invoices::customer_name.ilike(pattern)),
        );
    }

    let invoices: Vec<BillingInvoice> = q
        .order(billing_invoices::created_at.desc())
        .limit(limit)
        .offset(offset)
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(invoices))
}
pub async fn get_invoice(
    State(state): State<Arc<BillingApiState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<InvoiceWithItems>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let invoice: BillingInvoice = billing_invoices::table
        .filter(billing_invoices::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Invoice not found".to_string()))?;

    let items: Vec<BillingInvoiceItem> = billing_invoice_items::table
        .filter(billing_invoice_items::invoice_id.eq(id))
        .order(billing_invoice_items::sort_order.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(InvoiceWithItems { invoice, items }))
}
pub async fn update_invoice(
    State(state): State<Arc<BillingApiState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateInvoiceRequest>,
) -> Result<Json<BillingInvoice>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let now = Utc::now();

    diesel::update(billing_invoices::table.filter(billing_invoices::id.eq(id)))
        .set(billing_invoices::updated_at.eq(now))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    if let Some(customer_name) = req.customer_name {
        diesel::update(billing_invoices::table.filter(billing_invoices::id.eq(id)))
            .set(billing_invoices::customer_name.eq(customer_name))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    if let Some(customer_email) = req.customer_email {
        diesel::update(billing_invoices::table.filter(billing_invoices::id.eq(id)))
            .set(billing_invoices::customer_email.eq(customer_email))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    if let Some(notes) = req.notes {
        diesel::update(billing_invoices::table.filter(billing_invoices::id.eq(id)))
            .set(billing_invoices::notes.eq(notes))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    let invoice: BillingInvoice = billing_invoices::table
        .filter(billing_invoices::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Invoice not found".to_string()))?;

    Ok(Json(invoice))
}
pub async fn send_invoice(
    State(state): State<Arc<BillingApiState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<BillingInvoice>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let now = Utc::now();

    diesel::update(billing_invoices::table.filter(billing_invoices::id.eq(id)))
        .set((
            billing_invoices::status.eq("sent"),
            billing_invoices::sent_at.eq(Some(now)),
            billing_invoices::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    let invoice: BillingInvoice = billing_invoices::table
        .filter(billing_invoices::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Invoice not found".to_string()))?;

    Ok(Json(invoice))
}
pub async fn void_invoice(
    State(state): State<Arc<BillingApiState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<BillingInvoice>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let now = Utc::now();

    diesel::update(billing_invoices::table.filter(billing_invoices::id.eq(id)))
        .set((
            billing_invoices::status.eq("voided"),
            billing_invoices::voided_at.eq(Some(now)),
            billing_invoices::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    let invoice: BillingInvoice = billing_invoices::table
        .filter(billing_invoices::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Invoice not found".to_string()))?;

    Ok(Json(invoice))
}
pub async fn delete_invoice(
    State(state): State<Arc<BillingApiState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    diesel::delete(billing_invoices::table.filter(billing_invoices::id.eq(id)))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Delete error: {e}")))?;

    Ok(StatusCode::NO_CONTENT)
}
