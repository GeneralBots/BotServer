use super::*;

impl std::fmt::Display for KbPermissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(e) => write!(f, "Permission parse error: {e}"),
            Self::IoError(e) => write!(f, "IO error: {e}"),
            Self::InvalidPath(e) => write!(f, "Invalid path: {e}"),
        }
    }
}

impl std::error::Error for KbPermissionError {}

impl Default for KbPermissions {
    fn default() -> Self {
        Self {
            version: 1,
            default_access: AccessLevel::Authenticated,
            folders: HashMap::new(),
            inheritance: true,
        }
    }
}

pub fn create_default_permissions() -> KbPermissions {
    let mut folders = HashMap::new();

    folders.insert("public".to_string(), FolderPermission {
        access: AccessLevel::All,
        roles: Vec::new(),
        groups: Vec::new(),
        users: Vec::new(),
        index_visibility: Some(AccessLevel::All),
        inherit_parent: Some(true),
    });

    folders.insert("internal".to_string(), FolderPermission {
        access: AccessLevel::Authenticated,
        roles: Vec::new(),
        groups: Vec::new(),
        users: Vec::new(),
        index_visibility: Some(AccessLevel::Authenticated),
        inherit_parent: Some(true),
    });

    KbPermissions {
        version: 1,
        default_access: AccessLevel::Authenticated,
        folders,
        inheritance: true,
    }
}

pub fn generate_permissions_yaml(permissions: &KbPermissions) -> Result<String, KbPermissionError> {
    serde_json::to_string_pretty(permissions)
        .map_err(|e| KbPermissionError::ParseError(e.to_string()))
}

pub fn build_qdrant_permission_filter(user: &UserContext) -> serde_json::Value {
    if !user.is_authenticated {
        return serde_json::json!({
            "must": [
                { "key": "is_public", "match": { "value": true } }
            ]
        });
    }

    let user_id = user.user_id.to_string();
    let email = user.email.clone().unwrap_or_default();

    let mut should_conditions = vec![
        serde_json::json!({ "key": "is_public", "match": { "value": true } }),
        serde_json::json!({ "key": "access_level", "match": { "value": "authenticated" } }),
    ];

    if !user.roles.is_empty() {
        for role in &user.roles {
            should_conditions.push(serde_json::json!({
                "key": "allowed_roles",
                "match": { "any": [role] }
            }));
        }
    }

    if !user.groups.is_empty() {
        for group in &user.groups {
            should_conditions.push(serde_json::json!({
                "key": "allowed_groups",
                "match": { "any": [group] }
            }));
        }
    }

    should_conditions.push(serde_json::json!({
        "key": "allowed_users",
        "match": { "any": [user_id, email] }
    }));

    serde_json::json!({
        "should": should_conditions,
        "min_should": { "conditions": should_conditions.clone(), "min_count": 1 }
    })
}
