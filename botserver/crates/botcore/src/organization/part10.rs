use super::*;

impl UserAccessContext {
    pub fn anonymous() -> Self {
        Self {
            user_id: Uuid::nil(),
            is_authenticated: false,
            organization_id: None,
            roles: Vec::new(),
            groups: Vec::new(),
            permissions: Vec::new(),
        }
    }

    pub fn authenticated(user_id: Uuid, org_id: Option<Uuid>) -> Self {
        Self {
            user_id,
            is_authenticated: true,
            organization_id: org_id,
            roles: Vec::new(),
            groups: Vec::new(),
            permissions: Vec::new(),
        }
    }

    pub fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.roles = roles;
        self
    }

    pub fn with_groups(mut self, groups: Vec<String>) -> Self {
        self.groups = groups;
        self
    }

    pub fn with_permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
            || self.permissions.contains(&"*".to_string())
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string())
    }

    pub fn has_group(&self, group: &str) -> bool {
        self.groups.contains(&group.to_string())
    }
}
