use super::*;

impl ZitadelUser {
    pub fn to_authenticated_user(&self) -> Result<AuthenticatedUser, AuthError> {
        let user_id = Uuid::parse_str(&self.id).map_err(|_| {
            AuthError::InternalError(format!("Invalid user ID format: {}", self.id))
        })?;

        let username = if !self.username.is_empty() {
            self.username.clone()
        } else {
            self.email.clone().unwrap_or_else(|| self.id.clone())
        };

        let roles: Vec<Role> = self
            .roles
            .iter()
            .map(|r| map_zitadel_role_to_role(r))
            .collect();

        let roles = if roles.is_empty() {
            vec![Role::User]
        } else {
            roles
        };

        let mut user = AuthenticatedUser::new(user_id, username)
            .with_roles(roles);

        if let Some(ref email) = self.email {
            user = user.with_email(email);
        }

        if let Some(ref org_id) = self.organization_id {
            if let Ok(org_uuid) = Uuid::parse_str(org_id) {
                user = user.with_organization(org_uuid);
            }
        }

        for (key, value) in &self.metadata {
            user = user.with_metadata(key, value);
        }

        Ok(user)
    }
}
pub(super) fn map_zitadel_role_to_role(zitadel_role: &str) -> Role {
    let role_lower = zitadel_role.to_lowercase();

    if role_lower.contains("super") || role_lower.contains("root") {
        Role::SuperAdmin
    } else if role_lower.contains("admin") {
        Role::Admin
    } else if role_lower.contains("moderator") || role_lower.contains("mod") {
        Role::Moderator
    } else if role_lower.contains("bot_owner") || role_lower.contains("owner") {
        Role::BotOwner
    } else if role_lower.contains("bot_operator") || role_lower.contains("operator") {
        Role::BotOperator
    } else if role_lower.contains("bot_viewer") || role_lower.contains("viewer") {
        Role::BotViewer
    } else if role_lower.contains("service") {
        Role::Service
    } else if role_lower.contains("bot") && !role_lower.contains("_") {
        Role::Bot
    } else if role_lower.contains("user") || !role_lower.is_empty() {
        Role::User
    } else {
        Role::Anonymous
    }
}
