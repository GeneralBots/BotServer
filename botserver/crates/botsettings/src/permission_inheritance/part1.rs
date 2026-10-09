use super::*;

impl RoleHierarchy {
    pub fn new() -> Self {
        Self {
            roles: HashMap::new(),
            permission_cache: HashMap::new(),
        }
    }

    pub fn add_role(
        &mut self,
        name: &str,
        _display_name: &str,
        permissions: Vec<String>,
        parent_roles: Vec<String>,
        hierarchy_level: i32,
    ) {
        let node = RoleNode {
            permissions: permissions.into_iter().collect(),
            parent_roles,
            hierarchy_level,
        };
        self.roles.insert(name.to_string(), node);
        self.permission_cache.clear();
    }

    pub fn remove_role(&mut self, name: &str) {
        self.roles.remove(name);
        self.permission_cache.clear();
    }

    pub fn get_effective_permissions(&mut self, role_name: &str) -> HashSet<String> {
        if let Some(cached) = self.permission_cache.get(role_name) {
            return cached.clone();
        }

        let mut visited = HashSet::new();
        let permissions = self.resolve_permissions_recursive(role_name, &mut visited);
        self.permission_cache.insert(role_name.to_string(), permissions.clone());
        permissions
    }

    pub(super) fn resolve_permissions_recursive(
        &self,
        role_name: &str,
        visited: &mut HashSet<String>,
    ) -> HashSet<String> {
        if visited.contains(role_name) {
            return HashSet::new();
        }
        visited.insert(role_name.to_string());

        let Some(role) = self.roles.get(role_name) else {
            return HashSet::new();
        };

        let mut permissions = role.permissions.clone();

        for parent in &role.parent_roles {
            let parent_perms = self.resolve_permissions_recursive(parent, visited);
            permissions.extend(parent_perms);
        }

        permissions
    }

    pub fn can_manage(&self, manager_role: &str, target_role: &str) -> bool {
        let manager_level = self.roles.get(manager_role).map(|r| r.hierarchy_level).unwrap_or(0);
        let target_level = self.roles.get(target_role).map(|r| r.hierarchy_level).unwrap_or(0);
        manager_level > target_level
    }

    pub fn get_manageable_roles(&self, role_name: &str) -> Vec<String> {
        let manager_level = self.roles.get(role_name).map(|r| r.hierarchy_level).unwrap_or(0);
        self.roles
            .iter()
            .filter(|(_, node)| node.hierarchy_level < manager_level)
            .map(|(name, _)| name.clone())
            .collect()
    }
}
impl Default for RoleHierarchy {
    fn default() -> Self {
        Self::new()
    }
}
