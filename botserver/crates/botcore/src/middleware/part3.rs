use super::*;

impl RequestContext {
    pub fn new(user: AuthenticatedUser) -> Self {
        Self {
            user,
            organization: None,
            request_id: Uuid::new_v4(),
            client_ip: None,
            user_agent: None,
            bot_id: None,
            conversation_id: None,
        }
    }

    pub fn with_organization(mut self, org: OrganizationContext) -> Self {
        self.organization = Some(org);
        self
    }

    pub fn with_client_info(mut self, ip: Option<String>, user_agent: Option<String>) -> Self {
        self.client_ip = ip;
        self.user_agent = user_agent;
        self
    }

    pub fn with_bot(mut self, bot_id: Uuid) -> Self {
        self.bot_id = Some(bot_id);
        self
    }

    pub fn with_conversation(mut self, conversation_id: Uuid) -> Self {
        self.conversation_id = Some(conversation_id);
        self
    }

    /// Check if user can access a specific organization
    pub fn can_access_organization(&self, org_id: Uuid) -> bool {
        if self.user.is_super_admin {
            return true;
        }

        self.organization
            .as_ref()
            .map(|o| o.organization_id == org_id)
            .unwrap_or(false)
    }

    /// Check if user has permission within current organization
    pub fn has_org_permission(&self, permission: &str) -> bool {
        if self.user.is_super_admin {
            return true;
        }

        self.organization
            .as_ref()
            .map(|o| o.has_permission(permission))
            .unwrap_or(false)
    }

    /// Get organization ID if available
    pub fn org_id(&self) -> Option<Uuid> {
        self.organization.as_ref().map(|o| o.organization_id)
    }
}

// ============================================================================
// Middleware State
// ============================================================================
