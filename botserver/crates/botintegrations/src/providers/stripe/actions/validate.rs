use super::*;

/// Validates a Stripe object identifier such as `pi_...` before it is used
/// in a URL path segment.
pub(super) fn stripe_id(raw: &str, prefix: &str) -> Result<String, String> {
    let rest = raw
        .strip_prefix(prefix)
        .ok_or_else(|| invalid(format!("identifier must start with {prefix}")))?;
    if rest.is_empty()
        || raw.len() > 255
        || !rest.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(invalid(
            "identifier contains invalid characters".to_string(),
        ));
    }
    Ok(raw.to_string())
}

/// Validates an ISO currency code accepted by the payment intents API.
pub(super) fn validate_currency(code: &str) -> Result<(), String> {
    let ok = code.len() == 3 && code.chars().all(|c| c.is_ascii_lowercase());
    if ok {
        Ok(())
    } else {
        Err(invalid(
            "currency must be a three-letter lowercase ISO code".to_string(),
        ))
    }
}

/// Extracts a positive integer amount from the create/refund payload.
pub(super) fn positive_amount(object: &serde_json::Map<String, Value>) -> Result<Option<i64>, String> {
    match object.get("amount") {
        None | Some(Value::Null) => Ok(None),
        Some(value) => match value.as_i64() {
            Some(amount) if amount > 0 => Ok(Some(amount)),
            _ => Err(invalid(
                "amount must be a positive integer in the smallest currency unit".to_string(),
            )),
        },
    }
}
