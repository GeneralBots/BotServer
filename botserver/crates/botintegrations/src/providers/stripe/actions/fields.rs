use super::*;

/// Builds the customer creation fields from the catalog `data` object; at
/// least one of name or email must be provided.
pub(super) fn customer_create_fields(data: &Value) -> Result<Vec<(String, String)>, String> {
    let object = data.as_object().ok_or_else(|| {
        invalid("data must be a JSON object containing name and/or email".to_string())
    })?;
    let name = match object.get("name") {
        None | Some(Value::Null) => None,
        Some(Value::String(text)) if !text.trim().is_empty() => {
            let trimmed = text.trim();
            if trimmed.len() > MAX_NAME_LEN {
                return Err(invalid(format!(
                    "data.name must be at most {MAX_NAME_LEN} characters"
                )));
            }
            Some(trimmed.to_string())
        }
        Some(_) => return Err(invalid("data.name must be a string".to_string())),
    };
    let email = match object.get("email") {
        None | Some(Value::Null) => None,
        Some(Value::String(text)) => {
            let trimmed = text.trim();
            let valid =
                trimmed.contains('@') && !trimmed.is_empty() && trimmed.len() <= MAX_EMAIL_LEN;
            if !valid {
                return Err(invalid("data.email must be a valid address".to_string()));
            }
            Some(trimmed.to_string())
        }
        Some(_) => return Err(invalid("data.email must be a string".to_string())),
    };
    if name.is_none() && email.is_none() {
        return Err(invalid(
            "data must set at least one of name or email".to_string(),
        ));
    }
    let mut fields = Vec::new();
    if let Some(name) = name {
        fields.push(("name".to_string(), name));
    }
    if let Some(email) = email {
        fields.push(("email".to_string(), email));
    }
    Ok(fields)
}

/// Builds the payment intent creation fields from the catalog `data`
/// object; amount and currency are both required by the API.
pub(super) fn payment_intent_create_fields(data: &Value) -> Result<Vec<(String, String)>, String> {
    let object = data.as_object().ok_or_else(|| {
        invalid("data must be a JSON object containing amount and currency".to_string())
    })?;
    let amount = positive_amount(object)?.ok_or_else(|| {
        invalid("data.amount is required and must be a positive integer".to_string())
    })?;
    let currency = object
        .get("currency")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| invalid("data.currency is required".to_string()))?;
    validate_currency(currency)?;
    Ok(vec![
        ("amount".to_string(), amount.to_string()),
        ("currency".to_string(), currency.to_string()),
    ])
}
