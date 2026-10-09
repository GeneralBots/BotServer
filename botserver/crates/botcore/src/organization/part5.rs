use super::*;

impl Group {
    pub fn new(organization_id: Uuid, name: &str, display_name: &str) -> Self {
        let now = Utc::now();

        Self {
            id: Uuid::new_v4(),
            organization_id,
            name: name.to_string(),
            display_name: display_name.to_string(),
            description: None,
            permissions: Vec::new(),
            parent_group_id: None,
            is_system: false,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn as_system(mut self) -> Self {
        self.is_system = true;
        self
    }

    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }
}
