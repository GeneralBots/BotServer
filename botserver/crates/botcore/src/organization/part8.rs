use super::*;

impl BotAccessConfig {
    pub fn new(bot_id: Uuid, organization_id: Uuid) -> Self {
        let now = Utc::now();

        Self {
            bot_id,
            organization_id,
            visibility: BotVisibility::Private,
            allowed_roles: Vec::new(),
            allowed_groups: Vec::new(),
            allowed_users: Vec::new(),
            denied_users: Vec::new(),
            requires_authentication: true,
            ip_restrictions: Vec::new(),
            rate_limit_per_user: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn public() -> Self {
        Self {
            visibility: BotVisibility::Public,
            requires_authentication: false,
            ..Self::new(Uuid::nil(), Uuid::nil())
        }
    }

    pub fn check_access(&self, user: &UserAccessContext) -> AccessCheckResult {
        // Check if user is explicitly denied
        if self.denied_users.contains(&user.user_id) {
            return AccessCheckResult::Denied("User explicitly denied".to_string());
        }

        // Public bots
        if self.visibility == BotVisibility::Public {
            if self.requires_authentication && !user.is_authenticated {
                return AccessCheckResult::Denied("Authentication required".to_string());
            }
            return AccessCheckResult::Allowed;
        }

        // Must be authenticated for non-public bots
        if !user.is_authenticated {
            return AccessCheckResult::Denied("Authentication required".to_string());
        }

        // Check if user is explicitly allowed
        if self.allowed_users.contains(&user.user_id) {
            return AccessCheckResult::Allowed;
        }

        // Check role access
        if !self.allowed_roles.is_empty() {
            for role in &user.roles {
                if self.allowed_roles.contains(role) {
                    return AccessCheckResult::Allowed;
                }
            }
        }

        // Check group access
        if !self.allowed_groups.is_empty() {
            for group in &user.groups {
                if self.allowed_groups.contains(group) {
                    return AccessCheckResult::Allowed;
                }
            }
        }

        // Organization-wide access
        if self.visibility == BotVisibility::Organization
            && user.organization_id == Some(self.organization_id) {
                return AccessCheckResult::Allowed;
            }

        AccessCheckResult::Denied("Access not granted".to_string())
    }
}
