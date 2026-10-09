use super::*;

impl GroupHierarchy {
    pub fn new() -> Self {
        Self {
            groups: HashMap::new(),
            permission_cache: HashMap::new(),
        }
    }

    pub fn add_group(
        &mut self,
        name: &str,
        permissions: Vec<String>,
        parent_group: Option<String>,
    ) {
        if let Some(ref parent) = parent_group {
            if let Some(parent_node) = self.groups.get_mut(parent) {
                parent_node.child_groups.push(name.to_string());
            }
        }

        let node = GroupNode {
            permissions: permissions.into_iter().collect(),
            parent_group,
            child_groups: Vec::new(),
        };
        self.groups.insert(name.to_string(), node);
        self.permission_cache.clear();
    }

    pub fn remove_group(&mut self, name: &str) {
        if let Some(node) = self.groups.remove(name) {
            if let Some(parent) = &node.parent_group {
                if let Some(parent_node) = self.groups.get_mut(parent) {
                    parent_node.child_groups.retain(|c| c != name);
                }
            }
        }
        self.permission_cache.clear();
    }

    pub fn get_effective_permissions(&mut self, group_name: &str) -> HashSet<String> {
        if let Some(cached) = self.permission_cache.get(group_name) {
            return cached.clone();
        }

        let mut visited = HashSet::new();
        let permissions = self.resolve_permissions_recursive(group_name, &mut visited);
        self.permission_cache.insert(group_name.to_string(), permissions.clone());
        permissions
    }

    pub(super) fn resolve_permissions_recursive(
        &self,
        group_name: &str,
        visited: &mut HashSet<String>,
    ) -> HashSet<String> {
        if visited.contains(group_name) {
            return HashSet::new();
        }
        visited.insert(group_name.to_string());

        let Some(group) = self.groups.get(group_name) else {
            return HashSet::new();
        };

        let mut permissions = group.permissions.clone();

        if let Some(ref parent) = group.parent_group {
            let parent_perms = self.resolve_permissions_recursive(parent, visited);
            permissions.extend(parent_perms);
        }

        permissions
    }

    pub fn get_ancestors(&self, group_name: &str) -> Vec<String> {
        let mut ancestors = Vec::new();
        let mut current = group_name.to_string();

        while let Some(group) = self.groups.get(&current) {
            if let Some(ref parent) = group.parent_group {
                ancestors.push(parent.clone());
                current = parent.clone();
            } else {
                break;
            }
        }

        ancestors
    }

    pub fn get_descendants(&self, group_name: &str) -> Vec<String> {
        let mut descendants = Vec::new();
        let mut queue = vec![group_name.to_string()];

        while let Some(current) = queue.pop() {
            if let Some(group) = self.groups.get(&current) {
                for child in &group.child_groups {
                    descendants.push(child.clone());
                    queue.push(child.clone());
                }
            }
        }

        descendants
    }
}
impl Default for GroupHierarchy {
    fn default() -> Self {
        Self::new()
    }
}
