use super::*;

pub fn create_default_resolver() -> PermissionInheritanceResolver {
    PermissionInheritanceResolver::new()
        .with_default_roles()
        .with_default_groups()
}
