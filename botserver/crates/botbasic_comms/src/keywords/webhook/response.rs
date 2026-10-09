use super::*;

impl Default for WebhookResponse {
    fn default() -> Self {
        Self {
            status: 200,
            headers: std::collections::HashMap::new(),
            body: json!({"status": "ok"}),
        }
    }
}

impl WebhookResponse {
    pub fn success(data: Value) -> Self {
        Self {
            status: 200,
            headers: std::collections::HashMap::new(),
            body: data,
        }
    }

    pub fn error(status: u16, message: &str) -> Self {
        Self {
            status,
            headers: std::collections::HashMap::new(),
            body: json!({"error": message}),
        }
    }

    pub fn from_dynamic(value: &Dynamic) -> Self {
        if value.is_map() {
            let map = value.clone().try_cast::<rhai::Map>().unwrap_or_default();

            let status = map
                .get("status")
                .and_then(|v| v.as_int().ok())
                .unwrap_or(200) as u16;

            let body = map
                .get("body")
                .map(dynamic_to_json)
                .unwrap_or_else(|| json!({"status": "ok"}));

            let mut headers = std::collections::HashMap::new();
            if let Some(h) = map.get("headers") {
                if let Some(headers_map) = h.clone().try_cast::<rhai::Map>() {
                    for (k, v) in headers_map {
                        headers.insert(k.to_string(), v.to_string());
                    }
                }
            }

            Self {
                status,
                headers,
                body,
            }
        } else {
            Self::success(dynamic_to_json(value))
        }
    }
}
