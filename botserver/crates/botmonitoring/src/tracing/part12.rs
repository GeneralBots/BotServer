use super::*;

pub(super) fn attr_to_otlp_value(attr: &AttributeValue) -> serde_json::Value {
    match attr {
        AttributeValue::String(s) => serde_json::json!({"stringValue": s}),
        AttributeValue::Int(i) => serde_json::json!({"intValue": i.to_string()}),
        AttributeValue::Float(f) => serde_json::json!({"doubleValue": f}),
        AttributeValue::Bool(b) => serde_json::json!({"boolValue": b}),
        AttributeValue::StringArray(arr) => serde_json::json!({
            "arrayValue": {"values": arr.iter().map(|s| serde_json::json!({"stringValue": s})).collect::<Vec<_>>()}
        }),
        AttributeValue::IntArray(arr) => serde_json::json!({
            "arrayValue": {"values": arr.iter().map(|i| serde_json::json!({"intValue": i.to_string()})).collect::<Vec<_>>()}
        }),
    }
}
pub(super) fn jaeger_tags(s: &Span) -> Vec<serde_json::Value> {
    let mut tags = vec![
        serde_json::json!({"key": "span.kind", "type": "string", "value": format!("{:?}", s.kind).to_lowercase()}),
        serde_json::json!({"key": "service.name", "type": "string", "value": s.service_name}),
        serde_json::json!({"key": "status", "type": "string", "value": format!("{:?}", s.status)}),
    ];
    if let Some(msg) = &s.status_message {
        tags.push(serde_json::json!({"key": "status.message", "type": "string", "value": msg}));
    }
    if let Some(host) = &s.resource.host_name {
        tags.push(serde_json::json!({"key": "host.name", "type": "string", "value": host}));
    }
    for (k, v) in &s.attributes {
        let (typ, val) = match v {
            AttributeValue::String(s) => ("string", serde_json::json!(s)),
            AttributeValue::Int(i) => ("int64", serde_json::json!(i)),
            AttributeValue::Float(f) => ("float64", serde_json::json!(f)),
            AttributeValue::Bool(b) => ("bool", serde_json::json!(b)),
            AttributeValue::StringArray(arr) => ("string", serde_json::json!(arr.join(","))),
            AttributeValue::IntArray(arr) => ("int64", serde_json::json!(arr.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(","))),
        };
        tags.push(serde_json::json!({"key": k, "type": typ, "value": val}));
    }
    tags
}
pub(super) fn zipkin_tags(s: &Span) -> serde_json::Map<String, serde_json::Value> {
    let mut tags = serde_json::Map::new();
    tags.insert("span.kind".to_string(), serde_json::json!(format!("{:?}", s.kind).to_lowercase()));
    tags.insert("status".to_string(), serde_json::json!(format!("{:?}", s.status)));
    if let Some(msg) = &s.status_message {
        tags.insert("status.message".to_string(), serde_json::json!(msg));
    }
    if let Some(host) = &s.resource.host_name {
        tags.insert("host.name".to_string(), serde_json::json!(host));
    }
    for (k, v) in &s.attributes {
        let val = match v {
            AttributeValue::String(s) => serde_json::json!(s),
            AttributeValue::Int(i) => serde_json::json!(i),
            AttributeValue::Float(f) => serde_json::json!(f),
            AttributeValue::Bool(b) => serde_json::json!(b),
            AttributeValue::StringArray(arr) => serde_json::json!(arr.join(",")),
            AttributeValue::IntArray(arr) => serde_json::json!(arr.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",")),
        };
        tags.insert(k.clone(), val);
    }
    tags
}
