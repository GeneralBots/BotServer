use super::*;

impl PermissionInheritanceResolver {
    pub fn new() -> Self {
        Self {
            role_hierarchy: RoleHierarchy::new(),
            group_hierarchy: GroupHierarchy::new(),
            user_roles: HashMap::new(),
            user_groups: HashMap::new(),
            user_direct_permissions: HashMap::new(),
        }
    }

    pub fn with_default_roles(mut self) -> Self {
        self.role_hierarchy.add_role(
            "owner",
            "Owner",
            vec!["*".to_string()],
            Vec::new(),
            100,
        );

        self.role_hierarchy.add_role(
            "admin",
            "Administrator",
            vec![
                "org:manage".to_string(),
                "org:billing".to_string(),
                "org:members".to_string(),
                "bot:*".to_string(),
                "kb:*".to_string(),
                "app:*".to_string(),
            ],
            vec!["owner".to_string()],
            90,
        );

        self.role_hierarchy.add_role(
            "manager",
            "Manager",
            vec![
                "org:members:view".to_string(),
                "bot:create".to_string(),
                "bot:edit".to_string(),
                "bot:view".to_string(),
                "kb:read".to_string(),
                "kb:write".to_string(),
                "app:create".to_string(),
                "app:edit".to_string(),
            ],
            vec!["admin".to_string()],
            70,
        );

        self.role_hierarchy.add_role(
            "member",
            "Member",
            vec![
                "bot:view".to_string(),
                "kb:read".to_string(),
                "app:view".to_string(),
            ],
            vec!["manager".to_string()],
            50,
        );

        self.role_hierarchy.add_role(
            "viewer",
            "Viewer",
            vec![
                "bot:view".to_string(),
                "kb:read".to_string(),
            ],
            vec!["member".to_string()],
            30,
        );

        self.role_hierarchy.add_role(
            "guest",
            "Guest",
            vec!["bot:view:public".to_string()],
            Vec::new(),
            10,
        );

        self
    }

    pub fn with_default_groups(mut self) -> Self {
        self.group_hierarchy.add_group(
            "everyone",
            vec!["basic:access".to_string()],
            None,
        );

        self.group_hierarchy.add_group(
            "developers",
            vec![
                "bot:create".to_string(),
                "bot:edit".to_string(),
                "kb:write".to_string(),
            ],
            Some("everyone".to_string()),
        );

        self.group_hierarchy.add_group(
            "content_managers",
            vec![
                "kb:write".to_string(),
                "kb:admin".to_string(),
            ],
            Some("everyone".to_string()),
        );

        self.group_hierarchy.add_group(
            "support",
            vec![
                "bot:view".to_string(),
                "analytics:view".to_string(),
            ],
            Some("everyone".to_string()),
        );

        self
    }

    pub fn role_hierarchy_mut(&mut self) -> &mut RoleHierarchy {
        &mut self.role_hierarchy
    }

    pub fn group_hierarchy_mut(&mut self) -> &mut GroupHierarchy {
        &mut self.group_hierarchy
    }

    pub fn assign_role_to_user(&mut self, user_id: Uuid, role: &str) {
        self.user_roles
            .entry(user_id)
            .or_default()
            .insert(role.to_string());
    }

    pub fn remove_role_from_user(&mut self, user_id: Uuid, role: &str) {
        if let Some(roles) = self.user_roles.get_mut(&user_id) {
            roles.remove(role);
        }
    }

    pub fn add_user_to_group(&mut self, user_id: Uuid, group: &str) {
        self.user_groups
            .entry(user_id)
            .or_default()
            .insert(group.to_string());
    }

    pub fn remove_user_from_group(&mut self, user_id: Uuid, group: &str) {
        if let Some(groups) = self.user_groups.get_mut(&user_id) {
            groups.remove(group);
        }
    }

    pub fn add_direct_permission(&mut self, user_id: Uuid, permission: &str) {
        self.user_direct_permissions
            .entry(user_id)
            .or_default()
            .insert(permission.to_string());
    }

    pub fn remove_direct_permission(&mut self, user_id: Uuid, permission: &str) {
        if let Some(perms) = self.user_direct_permissions.get_mut(&user_id) {
            perms.remove(permission);
        }
    }

    pub fn get_effective_permissions(&mut self, user_id: Uuid) -> EffectivePermissions {
        let mut all_permissions = HashSet::new();
        let mut sources = Vec::new();

        if let Some(direct) = self.user_direct_permissions.get(&user_id) {
            for perm in direct {
                all_permissions.insert(perm.clone());
                sources.push(PermissionSource {
                    permission: perm.clone(),
                    source_type: PermissionSourceType::Direct,
                    source_name: "direct".to_string(),
                });
            }
        }

        if let Some(roles) = self.user_roles.get(&user_id).cloned() {
            for role in roles {
                let role_perms = self.role_hierarchy.get_effective_permissions(&role);
                for perm in role_perms {
                    if all_permissions.insert(perm.clone()) {
                        sources.push(PermissionSource {
                            permission: perm,
                            source_type: PermissionSourceType::Role,
                            source_name: role.clone(),
                        });
                    }
                }
            }
        }

        if let Some(groups) = self.user_groups.get(&user_id).cloned() {
            for group in groups {
                let group_perms = self.group_hierarchy.get_effective_permissions(&group);
                for perm in group_perms {
                    if all_permissions.insert(perm.clone()) {
                        sources.push(PermissionSource {
                            permission: perm,
                            source_type: PermissionSourceType::Group,
                            source_name: group.clone(),
                        });
                    }
                }
            }
        }

        EffectivePermissions {
            user_id,
            permissions: all_permissions,
            sources,
        }
    }

    pub fn check_permission(&mut self, user_id: Uuid, required_permission: &str) -> PermissionCheckResult {
        let effective = self.get_effective_permissions(user_id);

        if effective.permissions.contains("*") {
            return PermissionCheckResult {
                allowed: true,
                permission: required_permission.to_string(),
                source: Some(PermissionSource {
                    permission: "*".to_string(),
                    source_type: PermissionSourceType::Role,
                    source_name: "wildcard".to_string(),
                }),
            };
        }

        if effective.permissions.contains(required_permission) {
            let source = effective
                .sources
                .iter()
                .find(|s| s.permission == required_permission)
                .cloned();
            return PermissionCheckResult {
                allowed: true,
                permission: required_permission.to_string(),
                source,
            };
        }

        let parts: Vec<&str> = required_permission.split(':').collect();
        for i in (1..parts.len()).rev() {
            let wildcard = format!("{}:*", parts[..i].join(":"));
            if effective.permissions.contains(&wildcard) {
                let source = effective
                    .sources
                    .iter()
                    .find(|s| s.permission == wildcard)
                    .cloned();
                return PermissionCheckResult {
                    allowed: true,
                    permission: required_permission.to_string(),
                    source,
                };
            }
        }

        PermissionCheckResult {
            allowed: false,
            permission: required_permission.to_string(),
            source: None,
        }
    }

    pub fn get_user_roles(&self, user_id: Uuid) -> Vec<String> {
        self.user_roles
            .get(&user_id)
            .map(|r| r.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn get_user_groups(&self, user_id: Uuid) -> Vec<String> {
        self.user_groups
            .get(&user_id)
            .map(|g| g.iter().cloned().collect())
            .unwrap_or_default()
    }
}
impl Default for PermissionInheritanceResolver {
    fn default() -> Self {
        Self::new()
    }
}
