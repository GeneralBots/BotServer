use super::*;

impl AppAccessConfig {
    pub fn new(app_id: Uuid, app_type: AppType, organization_id: Uuid) -> Self {
        let now = Utc::now();

        Self {
            app_id,
            app_type,
            organization_id,
            visibility: AppVisibility::Private,
            allowed_roles: Vec::new(),
            allowed_groups: Vec::new(),
            allowed_users: Vec::new(),
            requires_authentication: true,
            submission_requires_auth: false,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn check_access(&self, user: &UserAccessContext, action: AppAction) -> AccessCheckResult {
        match action {
            AppAction::View => self.check_view_access(user),
            AppAction::Edit => self.check_edit_access(user),
            AppAction::Submit => self.check_submit_access(user),
            AppAction::Admin => self.check_admin_access(user),
        }
    }

    pub(super) fn check_view_access(&self, user: &UserAccessContext) -> AccessCheckResult {
        if self.visibility == AppVisibility::Public {
            if self.requires_authentication && !user.is_authenticated {
                return AccessCheckResult::Denied("Authentication required".to_string());
            }
            return AccessCheckResult::Allowed;
        }

        if !user.is_authenticated {
            return AccessCheckResult::Denied("Authentication required".to_string());
        }

        self.check_membership(user)
    }

    pub(super) fn check_edit_access(&self, user: &UserAccessContext) -> AccessCheckResult {
        if !user.is_authenticated {
            return AccessCheckResult::Denied("Authentication required".to_string());
        }

        // Check for edit permission in roles
        if user.has_permission("app:edit") {
            return self.check_membership(user);
        }

        AccessCheckResult::Denied("Edit permission required".to_string())
    }

    pub(super) fn check_submit_access(&self, user: &UserAccessContext) -> AccessCheckResult {
        if self.submission_requires_auth && !user.is_authenticated {
            return AccessCheckResult::Denied("Authentication required for submission".to_string());
        }

        // Public apps allow submissions
        if self.visibility == AppVisibility::Public {
            return AccessCheckResult::Allowed;
        }

        self.check_membership(user)
    }

    pub(super) fn check_admin_access(&self, user: &UserAccessContext) -> AccessCheckResult {
        if !user.is_authenticated {
            return AccessCheckResult::Denied("Authentication required".to_string());
        }

        if user.has_permission("app:admin") || user.has_permission("*") {
            return AccessCheckResult::Allowed;
        }

        AccessCheckResult::Denied("Admin permission required".to_string())
    }

    pub(super) fn check_membership(&self, user: &UserAccessContext) -> AccessCheckResult {
        // Explicit user access
        if self.allowed_users.contains(&user.user_id) {
            return AccessCheckResult::Allowed;
        }

        // Role access
        if !self.allowed_roles.is_empty() {
            for role in &user.roles {
                if self.allowed_roles.contains(role) {
                    return AccessCheckResult::Allowed;
                }
            }
        }

        // Group access
        if !self.allowed_groups.is_empty() {
            for group in &user.groups {
                if self.allowed_groups.contains(group) {
                    return AccessCheckResult::Allowed;
                }
            }
        }

        // Organization-wide
        if self.visibility == AppVisibility::Organization
            && user.organization_id == Some(self.organization_id) {
                return AccessCheckResult::Allowed;
            }

        AccessCheckResult::Denied("Access not granted".to_string())
    }
}
