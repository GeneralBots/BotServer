use super::*;

pub(super) fn extract_key_value(record: &Value, key_field: &str) -> Option<String> {
    record.get(key_field).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    })
}
pub(super) fn coerce_value(value: &Value, target_type: &FieldType) -> Result<Value, SynchronizeError> {
    match target_type {
        FieldType::String => match value {
            Value::String(_) => Ok(value.clone()),
            Value::Number(n) => Ok(Value::String(n.to_string())),
            Value::Bool(b) => Ok(Value::String(b.to_string())),
            Value::Null => Ok(Value::String(String::new())),
            _ => Ok(Value::String(value.to_string())),
        },
        FieldType::Integer => match value {
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(Value::Number(i.into()))
                } else {
                    Ok(value.clone())
                }
            }
            Value::String(s) => {
                let i: i64 = s
                    .parse()
                    .map_err(|_| SynchronizeError::TransformFailed(format!("Cannot parse '{}' as integer", s)))?;
                Ok(Value::Number(i.into()))
            }
            Value::Bool(b) => Ok(Value::Number(if *b { 1 } else { 0 }.into())),
            _ => Err(SynchronizeError::TransformFailed(format!(
                "Cannot coerce {:?} to integer",
                value
            ))),
        },
        FieldType::Float => match value {
            Value::Number(_) => Ok(value.clone()),
            Value::String(s) => {
                let f: f64 = s
                    .parse()
                    .map_err(|_| SynchronizeError::TransformFailed(format!("Cannot parse '{}' as float", s)))?;
                Ok(Value::Number(
                    serde_json::Number::from_f64(f).unwrap_or_else(|| 0.into()),
                ))
            }
            _ => Err(SynchronizeError::TransformFailed(format!(
                "Cannot coerce {:?} to float",
                value
            ))),
        },
        FieldType::Boolean => match value {
            Value::Bool(_) => Ok(value.clone()),
            Value::String(s) => {
                let b = matches!(s.to_lowercase().as_str(), "true" | "1" | "yes" | "on");
                Ok(Value::Bool(b))
            }
            Value::Number(n) => Ok(Value::Bool(n.as_i64().unwrap_or(0) != 0)),
            _ => Ok(Value::Bool(false)),
        },
        FieldType::DateTime => match value {
            Value::String(_) => Ok(value.clone()),
            Value::Number(n) => {
                if let Some(ts) = n.as_i64() {
                    if let Some(dt) = DateTime::from_timestamp(ts, 0) {
                        return Ok(Value::String(dt.to_rfc3339()));
                    }
                }
                Ok(value.clone())
            }
            _ => Ok(value.clone()),
        },
        FieldType::Json => Ok(value.clone()),
        FieldType::Uuid => match value {
            Value::String(s) => {
                let _ = Uuid::parse_str(s)
                    .map_err(|_| SynchronizeError::TransformFailed(format!("Invalid UUID: {}", s)))?;
                Ok(value.clone())
            }
            _ => Err(SynchronizeError::TransformFailed(format!(
                "Cannot coerce {:?} to UUID",
                value
            ))),
        },
    }
}
