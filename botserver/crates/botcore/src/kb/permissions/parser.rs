use super::*;

impl KbPermissionParser {
    pub fn new(permissions: KbPermissions) -> Self {
        Self {
            permissions,
            folder_cache: HashMap::new(),
        }
    }

    pub fn from_yaml(yaml_content: &str) -> Result<Self, KbPermissionError> {
        let permissions: KbPermissions = serde_json::from_str(yaml_content)
            .map_err(|e| KbPermissionError::ParseError(e.to_string()))?;
        Ok(Self::new(permissions))
    }

    pub async fn from_file(path: &Path) -> Result<Self, KbPermissionError> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| KbPermissionError::IoError(e.to_string()))?;
        Self::from_yaml(&content)
    }

    pub fn check_access(&mut self, folder_path: &str, user: &UserContext) -> PermissionCheckResult {
        let resolved = self.resolve_permission(folder_path);

        let (allowed, reason, matched_rule) = match resolved.access {
            AccessLevel::All => (true, "Public access".to_string(), Some("access: all".to_string())),
            AccessLevel::None => (false, "Access denied".to_string(), Some("access: none".to_string())),
            AccessLevel::Authenticated => {
                if user.is_authenticated {
                    (true, "Authenticated user".to_string(), Some("access: authenticated".to_string()))
                } else {
                    (false, "Authentication required".to_string(), Some("access: authenticated".to_string()))
                }
            }
            AccessLevel::RoleBased => {
                if !user.is_authenticated {
                    (false, "Authentication required".to_string(), Some("access: role_based".to_string()))
                } else if resolved.roles.is_empty() {
                    (true, "No roles required".to_string(), Some("access: role_based (empty)".to_string()))
                } else {
                    let has_role = user.roles.iter().any(|r| resolved.roles.contains(r));
                    if has_role {
                        let matched = user.roles.iter().find(|r| resolved.roles.contains(r));
                        (true, format!("Role matched: {:?}", matched), Some(format!("roles: {:?}", resolved.roles)))
                    } else {
                        (false, format!("Required roles: {:?}", resolved.roles), Some(format!("roles: {:?}", resolved.roles)))
                    }
                }
            }
            AccessLevel::GroupBased => {
                if !user.is_authenticated {
                    (false, "Authentication required".to_string(), Some("access: group_based".to_string()))
                } else if resolved.groups.is_empty() {
                    (true, "No groups required".to_string(), Some("access: group_based (empty)".to_string()))
                } else {
                    let has_group = user.groups.iter().any(|g| resolved.groups.contains(g));
                    if has_group {
                        let matched = user.groups.iter().find(|g| resolved.groups.contains(g));
                        (true, format!("Group matched: {:?}", matched), Some(format!("groups: {:?}", resolved.groups)))
                    } else {
                        (false, format!("Required groups: {:?}", resolved.groups), Some(format!("groups: {:?}", resolved.groups)))
                    }
                }
            }
            AccessLevel::UserBased => {
                if !user.is_authenticated {
                    (false, "Authentication required".to_string(), Some("access: user_based".to_string()))
                } else if resolved.users.is_empty() {
                    (false, "No users allowed".to_string(), Some("access: user_based (empty)".to_string()))
                } else {
                    let user_id_str = user.user_id.to_string();
                    let email_match = user.email.as_ref().map(|e| resolved.users.contains(e)).unwrap_or(false);
                    let id_match = resolved.users.contains(&user_id_str);

                    if email_match || id_match {
                        (true, "User authorized".to_string(), Some(format!("users: {:?}", resolved.users)))
                    } else {
                        (false, "User not in allowed list".to_string(), Some(format!("users: {:?}", resolved.users)))
                    }
                }
            }
        };

        let index_visible = self.check_index_visibility(&resolved.index_visibility, user);

        PermissionCheckResult {
            allowed,
            reason,
            matched_rule,
            index_visible,
        }
    }

    pub(super) fn check_index_visibility(&self, visibility: &AccessLevel, user: &UserContext) -> bool {
        match visibility {
            AccessLevel::All => true,
            AccessLevel::None => false,
            AccessLevel::Authenticated => user.is_authenticated,
            _ => user.is_authenticated,
        }
    }

    pub(super) fn resolve_permission(&mut self, folder_path: &str) -> ResolvedPermission {
        if let Some(cached) = self.folder_cache.get(folder_path) {
            return cached.clone();
        }

        let normalized_path = folder_path.trim_matches('/').to_lowercase();

        if let Some(folder_perm) = self.permissions.folders.get(&normalized_path) {
            let resolved = ResolvedPermission {
                access: folder_perm.access.clone(),
                roles: folder_perm.roles.clone(),
                groups: folder_perm.groups.clone(),
                users: folder_perm.users.clone(),
                index_visibility: folder_perm.index_visibility.clone().unwrap_or(folder_perm.access.clone()),
            };
            self.folder_cache.insert(folder_path.to_string(), resolved.clone());
            return resolved;
        }

        if self.permissions.inheritance {
            let parts: Vec<&str> = normalized_path.split('/').collect();
            for i in (0..parts.len()).rev() {
                let parent_path = parts[..i].join("/");
                if let Some(parent_perm) = self.permissions.folders.get(&parent_path) {
                    if parent_perm.inherit_parent.unwrap_or(true) {
                        let resolved = ResolvedPermission {
                            access: parent_perm.access.clone(),
                            roles: parent_perm.roles.clone(),
                            groups: parent_perm.groups.clone(),
                            users: parent_perm.users.clone(),
                            index_visibility: parent_perm.index_visibility.clone().unwrap_or(parent_perm.access.clone()),
                        };
                        self.folder_cache.insert(folder_path.to_string(), resolved.clone());
                        return resolved;
                    }
                }
            }
        }

        let default = ResolvedPermission {
            access: self.permissions.default_access.clone(),
            roles: Vec::new(),
            groups: Vec::new(),
            users: Vec::new(),
            index_visibility: self.permissions.default_access.clone(),
        };
        self.folder_cache.insert(folder_path.to_string(), default.clone());
        default
    }

    pub fn get_qdrant_filter_metadata(&mut self, folder_path: &str) -> QdrantPermissionMetadata {
        let resolved = self.resolve_permission(folder_path);

        QdrantPermissionMetadata {
            access_level: format!("{:?}", resolved.access).to_lowercase(),
            allowed_roles: resolved.roles,
            allowed_groups: resolved.groups,
            allowed_users: resolved.users,
            is_public: matches!(resolved.access, AccessLevel::All),
            requires_auth: !matches!(resolved.access, AccessLevel::All | AccessLevel::None),
        }
    }

    pub fn clear_cache(&mut self) {
        self.folder_cache.clear();
    }

    pub fn permissions(&self) -> &KbPermissions {
        &self.permissions
    }
}
