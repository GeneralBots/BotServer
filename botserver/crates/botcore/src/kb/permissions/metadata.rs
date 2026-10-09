use super::*;

impl QdrantPermissionMetadata {
    pub fn to_payload(&self) -> HashMap<String, serde_json::Value> {
        let mut payload = HashMap::new();
        payload.insert("access_level".to_string(), serde_json::json!(self.access_level));
        payload.insert("allowed_roles".to_string(), serde_json::json!(self.allowed_roles));
        payload.insert("allowed_groups".to_string(), serde_json::json!(self.allowed_groups));
        payload.insert("allowed_users".to_string(), serde_json::json!(self.allowed_users));
        payload.insert("is_public".to_string(), serde_json::json!(self.is_public));
        payload.insert("requires_auth".to_string(), serde_json::json!(self.requires_auth));
        payload
    }

    pub fn public() -> Self {
        Self {
            access_level: "all".to_string(),
            allowed_roles: Vec::new(),
            allowed_groups: Vec::new(),
            allowed_users: Vec::new(),
            is_public: true,
            requires_auth: false,
        }
    }

    pub fn authenticated_only() -> Self {
        Self {
            access_level: "authenticated".to_string(),
            allowed_roles: Vec::new(),
            allowed_groups: Vec::new(),
            allowed_users: Vec::new(),
            is_public: false,
            requires_auth: true,
        }
    }
}
