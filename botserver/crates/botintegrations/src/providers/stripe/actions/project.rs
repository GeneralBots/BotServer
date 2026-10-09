use super::*;

pub(super) fn project_balance_line(item: &Value) -> Value {
    json!({
        "amount": item.get("amount").cloned().unwrap_or_default(),
        "currency": item.get("currency").cloned().unwrap_or_default(),
    })
}

pub(super) fn project_customer(item: &Value) -> Value {
    json!({
        "id": item.get("id").cloned().unwrap_or_default(),
        "name": item.get("name").cloned().unwrap_or_default(),
        "email": item.get("email").cloned().unwrap_or_default(),
        "created": item.get("created").cloned().unwrap_or_default(),
    })
}

pub(super) fn project_payment_intent(item: &Value) -> Value {
    json!({
        "id": item.get("id").cloned().unwrap_or_default(),
        "amount": item.get("amount").cloned().unwrap_or_default(),
        "currency": item.get("currency").cloned().unwrap_or_default(),
        "status": item.get("status").cloned().unwrap_or_default(),
        "created": item.get("created").cloned().unwrap_or_default(),
    })
}

pub(super) fn project_price(item: &Value) -> Value {
    json!({
        "id": item.get("id").cloned().unwrap_or_default(),
        "unit_amount": item.get("unit_amount").cloned().unwrap_or_default(),
        "currency": item.get("currency").cloned().unwrap_or_default(),
        "active": item.get("active").cloned().unwrap_or(Value::Bool(true)),
        "product": item.get("product").cloned().unwrap_or_default(),
    })
}

pub(super) fn project_subscription(item: &Value) -> Value {
    json!({
        "id": item.get("id").cloned().unwrap_or_default(),
        "status": item.get("status").cloned().unwrap_or_default(),
        "customer": item.get("customer").cloned().unwrap_or_default(),
        "current_period_end": item.get("current_period_end").cloned().unwrap_or_default(),
    })
}

pub(super) fn projected_list(document: &Value, key: &str, project: fn(&Value) -> Value) -> Vec<Value> {
    document
        .get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().map(project).collect())
        .unwrap_or_default()
}

pub(super) async fn balance_retrieve(creds: &StripeCreds) -> Result<ActionOutcome, String> {
    let response = client::get(creds, "/balance").await?;
    response.require_success("balance.retrieve")?;
    let document = response.json("balance.retrieve")?;
    Ok(outcome(
        "Read current Stripe account balance".to_string(),
        json!({
            "available": document
                .get("available")
                .and_then(Value::as_array)
                .map(|items| items.iter().map(project_balance_line).collect::<Vec<_>>())
                .unwrap_or_default(),
            "pending": document
                .get("pending")
                .and_then(Value::as_array)
                .map(|items| items.iter().map(project_balance_line).collect::<Vec<_>>())
                .unwrap_or_default(),
        }),
    ))
}

pub(super) async fn customers_list(creds: &StripeCreds, params: &Value) -> Result<ActionOutcome, String> {
    let limit = bounded_limit(params, "limit", LIST_LIMIT_MAX, LIST_LIMIT_MAX)?;
    let response = client::get(creds, &format!("/customers?limit={limit}")).await?;
    response.require_success("customers.list")?;
    let document = response.json("customers.list")?;
    let has_more = document
        .get("has_more")
        .cloned()
        .unwrap_or(Value::Bool(false));
    let customers = projected_list(&document, "data", project_customer);
    let count = customers.len();
    Ok(outcome(
        format!("Listed {count} customers"),
        json!({ "customer_count": count, "has_more": has_more, "customers": customers }),
    ))
}

pub(super) async fn customers_search(creds: &StripeCreds, params: &Value) -> Result<ActionOutcome, String> {
    let email = required_email(params, "query")?;
    let limit = bounded_limit(params, "limit", 20, LIST_LIMIT_MAX)?;
    let mut path_query = format!("/customers?limit={limit}");
    push_query_pair(&mut path_query, "email", &email);
    let response = client::get(creds, &path_query).await?;
    response.require_success("customers.search")?;
    let document = response.json("customers.search")?;
    let customers = projected_list(&document, "data", project_customer);
    let count = customers.len();
    Ok(outcome(
        format!("Found {count} customers matching '{email}'"),
        json!({ "customer_count": count, "customers": customers }),
    ))
}
