use super::*;

impl OrganizationContext {
    pub fn new(organization_id: Uuid) -> Self {
        Self {
            organization_id,
            organization_name: None,
            plan_id: None,
            is_owner: false,
            permissions: Vec::new(),
        }
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.organization_name = Some(name);
        self
    }

    pub fn with_plan(mut self, plan_id: String) -> Self {
        self.plan_id = Some(plan_id);
        self
    }

    pub fn as_owner(mut self) -> Self {
        self.is_owner = true;
        self
    }

    pub fn with_permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn has_permission(&self, permission: &str) -> bool {
        self.is_owner || self.permissions.contains(&permission.to_string())
    }
}

// ============================================================================
// User Context (Authentication)
// ============================================================================
