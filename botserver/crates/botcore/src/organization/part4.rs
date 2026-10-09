use super::*;

impl Role {
    pub fn new(name: &str, display_name: &str, permissions: Vec<String>) -> Self {
        let now = Utc::now();

        Self {
            id: Uuid::new_v4(),
            organization_id: None,
            name: name.to_string(),
            display_name: display_name.to_string(),
            description: None,
            permissions,
            is_system: false,
            is_default: false,
            hierarchy_level: 100,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn system_role(name: &str, display_name: &str, permissions: Vec<String>, level: u32) -> Self {
        Self {
            is_system: true,
            hierarchy_level: level,
            ..Self::new(name, display_name, permissions)
        }
    }

    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
            || self.permissions.contains(&"*".to_string())
    }

    pub fn can_manage(&self, other: &Role) -> bool {
        self.hierarchy_level < other.hierarchy_level
    }
}
/// Default system roles
pub fn default_roles() -> Vec<Role> {
    vec![
        Role::system_role(
            "owner",
            "Owner",
            vec!["*".to_string()],
            0,
        ),
        Role::system_role(
            "admin",
            "Administrator",
            vec![
                "org:manage".to_string(),
                "org:members".to_string(),
                "org:settings".to_string(),
                "bot:*".to_string(),
                "kb:*".to_string(),
                "app:*".to_string(),
                "analytics:*".to_string(),
            ],
            10,
        ),
        Role::system_role(
            "manager",
            "Manager",
            vec![
                "org:members:view".to_string(),
                "bot:create".to_string(),
                "bot:edit".to_string(),
                "bot:delete".to_string(),
                "bot:publish".to_string(),
                "kb:read".to_string(),
                "kb:write".to_string(),
                "app:create".to_string(),
                "app:edit".to_string(),
                "app:delete".to_string(),
                "analytics:view".to_string(),
                "analytics:export".to_string(),
            ],
            20,
        ),
        Role::system_role(
            "member",
            "Member",
            vec![
                "bot:create".to_string(),
                "bot:edit".to_string(),
                "kb:read".to_string(),
                "kb:write".to_string(),
                "app:create".to_string(),
                "app:edit".to_string(),
                "analytics:view".to_string(),
            ],
            50,
        ),
        Role::system_role(
            "viewer",
            "Viewer",
            vec![
                "bot:view".to_string(),
                "kb:read".to_string(),
                "app:view".to_string(),
                "analytics:view".to_string(),
            ],
            80,
        ),
        Role::system_role(
            "guest",
            "Guest",
            vec![
                "kb:read".to_string(),
            ],
            90,
        ),
    ]
}

// ============================================================================
// Groups
// ============================================================================
