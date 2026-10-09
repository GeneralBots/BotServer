use super::*;

impl AuthenticatedUser {
    pub fn new(user_id: Uuid) -> Self {
        Self {
            user_id,
            email: None,
            display_name: None,
            roles: Vec::new(),
            groups: Vec::new(),
            organization_id: None,
            is_admin: false,
            is_super_admin: false,
            token_claims: None,
        }
    }

    pub fn anonymous() -> Self {
        Self {
            user_id: Uuid::nil(),
            email: None,
            display_name: Some("Anonymous".to_string()),
            roles: vec!["anonymous".to_string()],
            groups: Vec::new(),
            organization_id: None,
            is_admin: false,
            is_super_admin: false,
            token_claims: None,
        }
    }

    pub fn with_email(mut self, email: String) -> Self {
        self.email = Some(email);
        self
    }

    pub fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.roles = roles;
        self
    }

    pub fn with_groups(mut self, groups: Vec<String>) -> Self {
        self.groups = groups;
        self
    }

    pub fn with_organization(mut self, org_id: Uuid) -> Self {
        self.organization_id = Some(org_id);
        self
    }

    pub fn as_admin(mut self) -> Self {
        self.is_admin = true;
        self
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.is_super_admin || self.roles.contains(&role.to_string())
    }

    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        self.is_super_admin || roles.iter().any(|r| self.roles.contains(&r.to_string()))
    }

    pub fn has_group(&self, group: &str) -> bool {
        self.groups.contains(&group.to_string())
    }

    pub fn is_authenticated(&self) -> bool {
        !self.user_id.is_nil()
    }

    /// Convert to UserContext for KB permission checks
    #[cfg(any(feature = "research", feature = "llm"))]
    pub fn to_user_context(&self) -> UserContext {
        if self.is_authenticated() {
            UserContext::authenticated(self.user_id, self.email.clone(), self.organization_id)
                .with_roles(self.roles.clone())
                .with_groups(self.groups.clone())
        } else {
            UserContext::anonymous()
        }
    }

    /// Get Qdrant permission filter for this user
    #[cfg(any(feature = "research", feature = "llm"))]
    pub fn get_qdrant_filter(&self) -> serde_json::Value {
        build_qdrant_permission_filter(&self.to_user_context())
    }
}
