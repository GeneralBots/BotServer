use super::*;

/// Validates an email-shaped query parameter used for customer search.
pub(super) fn required_email(params: &Value, key: &str) -> Result<String, String> {
    let value = crate::providers::rest_client::required_text(params, key, MAX_EMAIL_LEN)?;
    if !value.contains('@') {
        return Err(invalid(format!("{key} must be a valid address")));
    }
    Ok(value)
}

pub(super) async fn customers_create(creds: &StripeCreds, params: &Value) -> Result<ActionOutcome, String> {
    let data = params
        .get("data")
        .ok_or_else(|| invalid("data is required".to_string()))?;
    let fields = customer_create_fields(data)?;
    let response = client::post_form(creds, "/customers", fields).await?;
    response.require_success("customers.create")?;
    let document = response.json("customers.create")?;
    let id = document.get("id").cloned().unwrap_or_default();
    Ok(outcome(
        format!("Created customer {id}"),
        json!({ "customer": project_customer(&document) }),
    ))
}

pub(super) async fn payment_intents_list(
    creds: &StripeCreds,
    params: &Value,
) -> Result<ActionOutcome, String> {
    let limit = bounded_limit(params, "limit", LIST_LIMIT_MAX, LIST_LIMIT_MAX)?;
    let response = client::get(creds, &format!("/payment_intents?limit={limit}")).await?;
    response.require_success("payment_intents.list")?;
    let document = response.json("payment_intents.list")?;
    let payments = projected_list(&document, "data", project_payment_intent);
    let count = payments.len();
    Ok(outcome(
        format!("Listed {count} payments"),
        json!({ "payment_count": count, "payments": payments }),
    ))
}

pub(super) async fn payment_intents_get(creds: &StripeCreds, params: &Value) -> Result<ActionOutcome, String> {
    let raw_id = crate::providers::rest_client::required_text(params, "resource_id", 255)?;
    let id = stripe_id(&raw_id, "pi_")?;
    let response = client::get(creds, &format!("/payment_intents/{id}")).await?;
    response.require_success("payment_intents.get")?;
    let document = response.json("payment_intents.get")?;
    let status = document.get("status").cloned().unwrap_or_default();
    Ok(outcome(
        format!("Read payment {id} ({status})"),
        json!({ "payment": project_payment_intent(&document) }),
    ))
}

pub(super) async fn payment_intents_create(
    creds: &StripeCreds,
    params: &Value,
) -> Result<ActionOutcome, String> {
    let data = params
        .get("data")
        .ok_or_else(|| invalid("data is required".to_string()))?;
    let fields = payment_intent_create_fields(data)?;
    let response = client::post_form(creds, "/payment_intents", fields).await?;
    response.require_success("payment_intents.create")?;
    let document = response.json("payment_intents.create")?;
    let id = document.get("id").cloned().unwrap_or_default();
    let status = document.get("status").cloned().unwrap_or_default();
    Ok(outcome(
        format!("Created payment intent {id} ({status})"),
        json!({ "payment": project_payment_intent(&document) }),
    ))
}

pub(super) async fn prices_list(creds: &StripeCreds, params: &Value) -> Result<ActionOutcome, String> {
    let limit = bounded_limit(params, "limit", LIST_LIMIT_MAX, LIST_LIMIT_MAX)?;
    let response = client::get(creds, &format!("/prices?limit={limit}&active=true")).await?;
    response.require_success("prices.list")?;
    let document = response.json("prices.list")?;
    let prices = projected_list(&document, "data", project_price);
    let count = prices.len();
    Ok(outcome(
        format!("Listed {count} active prices"),
        json!({ "price_count": count, "prices": prices }),
    ))
}

pub(super) async fn refunds_create(creds: &StripeCreds, params: &Value) -> Result<ActionOutcome, String> {
    let raw_intent = crate::providers::rest_client::required_text(params, "payment_intent", 255)?;
    let intent = stripe_id(&raw_intent, "pi_")?;
    let reason = optional_text(params, "reason", 64)?;
    if let Some(reason) = &reason {
        if reason != "duplicate" && reason != "fraudulent" && reason != "requested_by_customer" {
            return Err(invalid(
                "reason must be duplicate, fraudulent or requested_by_customer".to_string(),
            ));
        }
    }
    // The catalog models the partial refund amount as an integer parameter,
    // so reuse the shared bound check against a synthetic object view.
    let amount = match params.get("amount") {
        None | Some(Value::Null) => None,
        Some(value) => {
            let mut synthetic = serde_json::Map::new();
            synthetic.insert("amount".to_string(), value.clone());
            positive_amount(&synthetic)?
        }
    };

    let mut fields = vec![("payment_intent".to_string(), intent.clone())];
    if let Some(amount) = amount {
        fields.push(("amount".to_string(), amount.to_string()));
    }
    if let Some(reason) = reason {
        fields.push(("reason".to_string(), reason));
    }
    let response = client::post_form(creds, "/refunds", fields).await?;
    response.require_success("refunds.create")?;
    let document = response.json("refunds.create")?;
    let refund_id = document.get("id").cloned().unwrap_or_default();
    let refund_amount = document.get("amount").cloned().unwrap_or_default();
    let status = document.get("status").cloned().unwrap_or_default();
    Ok(outcome(
        format!("Refunded {refund_amount} from {intent} (refund {refund_id}, {status})"),
        json!({
            "refund": {
                "id": refund_id,
                "amount": refund_amount,
                "status": status,
                "payment_intent": intent,
            }
        }),
    ))
}

pub(super) async fn subscriptions_list(creds: &StripeCreds, params: &Value) -> Result<ActionOutcome, String> {
    let limit = bounded_limit(params, "limit", LIST_LIMIT_MAX, LIST_LIMIT_MAX)?;
    let mut path_query = format!("/subscriptions?limit={limit}");
    push_query_pair(&mut path_query, "status", "all");
    let response = client::get(creds, &path_query).await?;
    response.require_success("subscriptions.list")?;
    let document = response.json("subscriptions.list")?;
    let subscriptions = projected_list(&document, "data", project_subscription);
    let count = subscriptions.len();
    Ok(outcome(
        format!("Listed {count} subscriptions"),
        json!({ "subscription_count": count, "subscriptions": subscriptions }),
    ))
}

/// Entry point used by [`super::StripeAdapter`]; unknown keys are rejected
/// here before any parameter validation or network activity happens.
pub(crate) async fn invoke(
    action: &str,
    creds: &StripeCreds,
    params: &Value,
) -> Result<ActionOutcome, String> {
    match action {
        "balance.retrieve" => balance_retrieve(creds).await,
        "customers.list" => customers_list(creds, params).await,
        "customers.search" => customers_search(creds, params).await,
        "customers.create" => customers_create(creds, params).await,
        "payment_intents.list" => payment_intents_list(creds, params).await,
        "payment_intents.get" => payment_intents_get(creds, params).await,
        "payment_intents.create" => payment_intents_create(creds, params).await,
        "prices.list" => prices_list(creds, params).await,
        "refunds.create" => refunds_create(creds, params).await,
        "subscriptions.list" => subscriptions_list(creds, params).await,
        _ => Err(crate::providers::ERR_UNKNOWN_ACTION.to_string()),
    }
}
