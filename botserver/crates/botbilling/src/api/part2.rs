use super::*;

pub async fn record_payment(
    State(state): State<Arc<BillingApiState>>,
    Json(req): Json<RecordPaymentRequest>,
) -> Result<Json<BillingPayment>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = get_bot_context(&state.pool, &state.get_default_bot);
    let id = Uuid::new_v4();
    let now = Utc::now();
    let payment_number = generate_payment_number(&mut conn, branch_id);

    let payment = BillingPayment {
        id,
        branch_id,
        invoice_id: req.invoice_id,
        amount: bd(req.amount),
        currency: Some("USD".to_string()),
        payment_method: Some(req.payment_method.clone().unwrap_or_else(|| "other".to_string())),
        status: Some("completed".to_string()),
        paid_at: Some(now),
        gateway_response: None,
        created_at: now,
        updated_at: now,
        payment_number,
        payment_reference: req.payment_reference.clone(),
        payer_name: req.payer_name.clone(),
        payer_email: req.payer_email.clone(),
        notes: req.notes.clone(),
        refunded_at: None,
        refund_amount: None,
    };

    diesel::insert_into(billing_payments::table)
        .values(&payment)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    if let Some(invoice_id) = req.invoice_id {
        let invoice: BillingInvoice = billing_invoices::table
            .filter(billing_invoices::id.eq(invoice_id))
            .first(&mut conn)
            .map_err(|_| (StatusCode::NOT_FOUND, "Invoice not found".to_string()))?;

        let new_paid = &invoice.amount_paid + bd(req.amount);
        let total_val = invoice.total.unwrap_or_else(|| bd(0.0));
        let new_due = &total_val - &new_paid;

        let new_status = if bd_to_f64(&new_due) <= 0.0 {
            "paid"
        } else if bd_to_f64(&new_paid) > 0.0 {
            "partial"
        } else {
            invoice.status.as_deref().unwrap_or("")
        };

        let paid_at = if new_status == "paid" { Some(now) } else { invoice.paid_at };

        diesel::update(billing_invoices::table.filter(billing_invoices::id.eq(invoice_id)))
            .set((
                billing_invoices::amount_paid.eq(new_paid),
                billing_invoices::amount_due.eq(new_due),
                billing_invoices::status.eq(new_status),
                billing_invoices::paid_at.eq(paid_at),
                billing_invoices::updated_at.eq(now),
            ))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    Ok(Json(payment))
}
pub async fn list_payments(
    State(state): State<Arc<BillingApiState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<BillingPayment>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = get_bot_context(&state.pool, &state.get_default_bot);
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let mut q = billing_payments::table
        .filter(billing_payments::branch_id.eq(branch_id))
        .into_boxed();

    if let Some(status) = query.status {
        if status != "all" {
            q = q.filter(billing_payments::status.eq(status));
        }
    }

    let payments: Vec<BillingPayment> = q
        .order(billing_payments::paid_at.desc())
        .limit(limit)
        .offset(offset)
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(payments))
}
pub async fn get_payment(
    State(state): State<Arc<BillingApiState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<BillingPayment>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let payment: BillingPayment = billing_payments::table
        .filter(billing_payments::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Payment not found".to_string()))?;

    Ok(Json(payment))
}
pub async fn create_quote(
    State(state): State<Arc<BillingApiState>>,
    Json(req): Json<CreateQuoteRequest>,
) -> Result<Json<BillingQuote>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = get_bot_context(&state.pool, &state.get_default_bot);
    let id = Uuid::new_v4();
    let now = Utc::now();
    let quote_number = generate_quote_number(&mut conn, branch_id);

    let issue_date = req
        .issue_date
        .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
        .unwrap_or_else(|| now.date_naive());

    let valid_until = req
        .valid_until
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

    let quote = BillingQuote {
        id,
        branch_id,
        quote_number,
        customer_name: req.customer_name.clone(),
        customer_email: req.customer_email.clone(),
        items: None,
        total: Some(total),
        currency: Some(req.currency.clone().unwrap_or_else(|| "USD".to_string())),
        status: Some("draft".to_string()),
        valid_until: Some(valid_until),
        notes: req.notes.clone(),
        created_at: now,
        updated_at: now,
        customer_id: req.customer_id,
        customer_address: req.customer_address.clone(),
        issue_date,
        subtotal,
        tax_rate,
        tax_amount,
        discount_percent,
        discount_amount,
        terms: req.terms.clone(),
        accepted_at: None,
        rejected_at: None,
        converted_invoice_id: None,
        sent_at: None,
    };

    diesel::insert_into(billing_quotes::table)
        .values(&quote)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    for (idx, item) in items.iter().enumerate() {
        let item_amount = item.quantity * item.unit_price;
        let item_discount = item_amount * item.discount_percent.unwrap_or(0.0) / 100.0;
        let final_amount = item_amount - item_discount;

        let quote_item = BillingQuoteItem {
            id: Uuid::new_v4(),
            quote_id: id,
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

        diesel::insert_into(billing_quote_items::table)
            .values(&quote_item)
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert item error: {e}")))?;
    }

    Ok(Json(quote))
}
pub async fn list_quotes(
    State(state): State<Arc<BillingApiState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<BillingQuote>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = get_bot_context(&state.pool, &state.get_default_bot);
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let mut q = billing_quotes::table
        .filter(billing_quotes::branch_id.eq(branch_id))
        .into_boxed();

    if let Some(status) = query.status {
        if status != "all" {
            q = q.filter(billing_quotes::status.eq(status));
        }
    }

    if let Some(search) = query.search {
        let pattern = format!("%{search}%");
        q = q.filter(
            billing_quotes::quote_number
                .ilike(pattern.clone())
                .or(billing_quotes::customer_name.ilike(pattern)),
        );
    }

    let quotes: Vec<BillingQuote> = q
        .order(billing_quotes::created_at.desc())
        .limit(limit)
        .offset(offset)
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(quotes))
}
pub async fn get_quote(
    State(state): State<Arc<BillingApiState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<QuoteWithItems>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let quote: BillingQuote = billing_quotes::table
        .filter(billing_quotes::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Quote not found".to_string()))?;

    let items: Vec<BillingQuoteItem> = billing_quote_items::table
        .filter(billing_quote_items::quote_id.eq(id))
        .order(billing_quote_items::sort_order.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(QuoteWithItems { quote, items }))
}
