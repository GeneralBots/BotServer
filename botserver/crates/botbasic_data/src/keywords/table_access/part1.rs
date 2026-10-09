use super::*;

impl UserRoles {
    pub fn new(roles: Vec<String>) -> Self {
        Self {
            roles: roles.into_iter().map(|r| r.to_lowercase()).collect(),
            user_id: None,
        }
    }

    pub fn with_user_id(roles: Vec<String>, user_id: uuid::Uuid) -> Self {
        Self {
            roles: roles.into_iter().map(|r| r.to_lowercase()).collect(),
            user_id: Some(user_id),
        }
    }

    pub fn anonymous() -> Self {
        Self::default()
    }

    pub fn from_user_session(session: &UserSession) -> Self {
        let mut roles = Vec::new();

        // Try different keys where roles might be stored
        let role_keys = ["roles", "user_roles", "zitadel_roles"];

        for key in role_keys {
            if let Some(value) = session.context_data.get(key) {
                match value {
                    // Array of strings
                    Value::Array(arr) => {
                        for item in arr {
                            if let Value::String(s) = item {
                                roles.push(s.trim().to_lowercase());
                            }
                        }
                        if !roles.is_empty() {
                            break;
                        }
                    }
                    // Semicolon-separated string
                    Value::String(s) => {
                        roles = s
                            .split(';')
                            .map(|r| r.trim().to_lowercase())
                            .filter(|r| !r.is_empty())
                            .collect();
                        if !roles.is_empty() {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }

        // Also check if user is marked as admin in context
        if matches!(session.context_data.get("is_admin"), Some(Value::Bool(true)))
            && !roles.contains(&"admin".to_string())
        {
            roles.push("admin".to_string());
        }

        Self {
            roles,
            user_id: Some(session.user_id),
        }
    }

    pub fn has_access(&self, required_roles: &[String]) -> bool {
        if required_roles.is_empty() {
            return true; // No roles specified = everyone has access
        }

        // Check if user has any of the required roles
        self.roles.iter().any(|user_role| {
            required_roles
                .iter()
                .any(|req| req.to_lowercase() == *user_role)
        })
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == &role.to_lowercase())
    }

    pub fn is_admin(&self) -> bool {
        self.has_role("admin")
    }
}
