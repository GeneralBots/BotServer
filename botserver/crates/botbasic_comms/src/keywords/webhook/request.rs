use super::*;

impl WebhookRequest {
    pub fn new(
        method: &str,
        headers: std::collections::HashMap<String, String>,
        query_params: std::collections::HashMap<String, String>,
        body: Value,
        path: &str,
    ) -> Self {
        Self {
            method: method.to_string(),
            headers,
            query_params,
            body,
            path: path.to_string(),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn to_dynamic(&self) -> Dynamic {
        let mut map = rhai::Map::new();

        map.insert("method".into(), Dynamic::from(self.method.clone()));
        map.insert("path".into(), Dynamic::from(self.path.clone()));
        map.insert(
            "timestamp".into(),
            Dynamic::from(self.timestamp.to_rfc3339()),
        );

        let mut headers_map = rhai::Map::new();
        for (k, v) in &self.headers {
            headers_map.insert(k.clone().into(), Dynamic::from(v.clone()));
        }
        map.insert("headers".into(), Dynamic::from(headers_map));

        let mut params_map = rhai::Map::new();
        for (k, v) in &self.query_params {
            params_map.insert(k.clone().into(), Dynamic::from(v.clone()));
        }
        map.insert("params".into(), Dynamic::from(params_map));

        map.insert("body".into(), json_to_dynamic(&self.body));

        Dynamic::from(map)
    }
}
