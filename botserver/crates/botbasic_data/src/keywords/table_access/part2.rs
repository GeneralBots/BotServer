use super::*;

impl TableAccessInfo {
    pub fn can_read(&self, user_roles: &UserRoles) -> bool {
        user_roles.has_access(&self.read_roles)
    }

    pub fn can_write(&self, user_roles: &UserRoles) -> bool {
        user_roles.has_access(&self.write_roles)
    }

    pub fn can_read_field(&self, field_name: &str, user_roles: &UserRoles) -> bool {
        if let Some(field_roles) = self.field_read_roles.get(field_name) {
            user_roles.has_access(field_roles)
        } else {
            true // No field-level restriction
        }
    }

    pub fn can_write_field(&self, field_name: &str, user_roles: &UserRoles) -> bool {
        if let Some(field_roles) = self.field_write_roles.get(field_name) {
            user_roles.has_access(field_roles)
        } else {
            true // No field-level restriction
        }
    }

    pub fn get_restricted_read_fields(&self, user_roles: &UserRoles) -> Vec<String> {
        self.field_read_roles
            .iter()
            .filter(|(_, roles)| !user_roles.has_access(roles))
            .map(|(field, _)| field.clone())
            .collect()
    }

    pub fn get_restricted_write_fields(&self, user_roles: &UserRoles) -> Vec<String> {
        self.field_write_roles
            .iter()
            .filter(|(_, roles)| !user_roles.has_access(roles))
            .map(|(field, _)| field.clone())
            .collect()
    }
}
