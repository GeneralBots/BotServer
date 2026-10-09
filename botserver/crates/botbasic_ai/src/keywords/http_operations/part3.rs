use super::*;

pub(super) async fn execute_graphql(
    endpoint: &str,
    query: &str,
    variables: Value,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let graphql_body = json!({
        "query": query,
        "variables": variables
    });

    execute_http_request(Method::POST, endpoint, Some(graphql_body), None).await
}
pub(super) async fn execute_soap(
    endpoint: &str,
    operation: &str,
    params: Value,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    use std::fmt::Write;

    let soap_body = build_soap_envelope(operation, &params);

    let mut headers = HashMap::new();
    headers.insert(
        "Content-Type".to_string(),
        "text/xml; charset=utf-8".to_string(),
    );
    let mut soap_action = String::new();
    let _ = write!(soap_action, "\"{operation}\"");
    headers.insert("SOAPAction".to_string(), soap_action);

    let client = Client::builder()
        .timeout(Duration::from_secs(120))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| {
            log::error!("Failed to build HTTP client: {e}");
            e
        })?;

    let mut header_map = HeaderMap::new();
    for (name, value) in headers {
        if let (Ok(header_name), Ok(header_value)) = (
            HeaderName::try_from(name.as_str()),
            HeaderValue::try_from(value.as_str()),
        ) {
            header_map.insert(header_name, header_value);
        }
    }

    let response = client
        .post(endpoint)
        .headers(header_map)
        .body(soap_body)
        .send()
        .await
        .map_err(|e| {
            log::error!("SOAP request failed for endpoint {endpoint}: {e}");
            e
        })?;

    let status = response.status();
    let body_text = response.text().await.unwrap_or_default();

    let parsed_response = parse_soap_response(&body_text);

    trace!("SOAP request to {endpoint} completed with status: {status}");

    Ok(json!({
        "status": status.as_u16(),
        "data": parsed_response
    }))
}
pub(super) fn build_soap_envelope(operation: &str, params: &Value) -> String {
    use std::fmt::Write;

    let mut params_xml = String::new();

    if let Value::Object(obj) = params {
        for (key, value) in obj {
            let value_str = match value {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => value.to_string(),
            };
            let _ = write!(params_xml, "<{key}>{value_str}</{key}>");
        }
    }

    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/">
    <soap:Body>
        <{operation} xmlns="http://tempuri.org/">
            {params_xml}
        </{operation}>
    </soap:Body>
</soap:Envelope>"#
    )
}
pub(super) fn parse_soap_response(xml: &str) -> Value {
    if let Some(body_start) = xml.find("<soap:Body>") {
        if let Some(body_end) = xml.find("</soap:Body>") {
            let body_content = &xml[body_start + 11..body_end];
            return json!({
                "raw": body_content.trim(),
                "xml": xml
            });
        }
    }

    if let Some(body_start) = xml.find("<soap12:Body>") {
        if let Some(body_end) = xml.find("</soap12:Body>") {
            let body_content = &xml[body_start + 13..body_end];
            return json!({
                "raw": body_content.trim(),
                "xml": xml
            });
        }
    }

    json!({
        "raw": xml,
        "xml": xml
    })
}
pub(super) fn dynamic_to_json(value: &Dynamic) -> Value {
    if value.is_unit() {
        Value::Null
    } else if value.is_bool() {
        Value::Bool(value.as_bool().unwrap_or(false))
    } else if value.is_int() {
        Value::Number(value.as_int().unwrap_or(0).into())
    } else if value.is_float() {
        if let Ok(f) = value.as_float() {
            serde_json::Number::from_f64(f)
                .map(Value::Number)
                .unwrap_or(Value::Null)
        } else {
            Value::Null
        }
    } else if value.is_string() {
        Value::String(value.to_string())
    } else if value.is_array() {
        let arr = value.clone().into_array().unwrap_or_default();
        Value::Array(arr.iter().map(dynamic_to_json).collect())
    } else if value.is_map() {
        let map = value.clone().try_cast::<Map>().unwrap_or_default();
        let obj: serde_json::Map<String, Value> = map
            .iter()
            .map(|(k, v)| (k.to_string(), dynamic_to_json(v)))
            .collect();
        Value::Object(obj)
    } else {
        Value::String(value.to_string())
    }
}
pub(super) fn json_to_dynamic(value: &Value) -> Dynamic {
    match value {
        Value::Null => Dynamic::UNIT,
        Value::Bool(b) => Dynamic::from(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from(f)
            } else {
                Dynamic::UNIT
            }
        }
        Value::String(s) => Dynamic::from(s.clone()),
        Value::Array(arr) => {
            let rhai_arr: rhai::Array = arr.iter().map(json_to_dynamic).collect();
            Dynamic::from(rhai_arr)
        }
        Value::Object(obj) => {
            let mut map = Map::new();
            for (k, v) in obj {
                map.insert(k.clone().into(), json_to_dynamic(v));
            }
            Dynamic::from(map)
        }
    }
}
