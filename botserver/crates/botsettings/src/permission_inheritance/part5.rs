use super::*;

impl ResourcePermission {
    pub fn check_access(
        &self,
        user_id: Uuid,
        user_roles: &[String],
        user_groups: &[String],
        is_authenticated: bool,
    ) -> bool {
        if self.denied_users.contains(&user_id) {
            return false;
        }

        if self.owner_id == user_id {
            return true;
        }

        if self.allowed_users.contains(&user_id) {
            return true;
        }

        match self.visibility {
            ResourceVisibility::Public => true,
            ResourceVisibility::Organization => {
                if !is_authenticated {
                    return false;
                }

                if !self.allowed_roles.is_empty()
                    && !user_roles.iter().any(|r| self.allowed_roles.contains(r))
                {
                    return false;
                }

                if !self.allowed_groups.is_empty()
                    && !user_groups.iter().any(|g| self.allowed_groups.contains(g))
                {
                    return false;
                }

                true
            }
            ResourceVisibility::Private => false,
        }
    }
}
