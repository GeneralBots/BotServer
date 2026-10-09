//! Knowledge base permission model, parser and Qdrant filter helpers.

pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::path::Path;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KbPermissions {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub default_access: AccessLevel,
    #[serde(default)]
    pub folders: HashMap<String, FolderPermission>,
    #[serde(default = "default_true")]
    pub inheritance: bool,
}

fn default_version() -> u32 {
    1
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AccessLevel {
    All,
    #[default]
    Authenticated,
    RoleBased,
    GroupBased,
    UserBased,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderPermission {
    #[serde(default)]
    pub access: AccessLevel,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub groups: Vec<String>,
    #[serde(default)]
    pub users: Vec<String>,
    #[serde(default)]
    pub index_visibility: Option<AccessLevel>,
    #[serde(default)]
    pub inherit_parent: Option<bool>,
}

impl Default for FolderPermission {
    fn default() -> Self {
        Self {
            access: AccessLevel::Authenticated,
            roles: Vec::new(),
            groups: Vec::new(),
            users: Vec::new(),
            index_visibility: None,
            inherit_parent: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserContext {
    pub user_id: Uuid,
    pub email: Option<String>,
    pub roles: Vec<String>,
    pub groups: Vec<String>,
    pub is_authenticated: bool,
    pub organization_id: Option<Uuid>,
}

impl UserContext {
    pub fn anonymous() -> Self {
        Self {
            user_id: Uuid::nil(),
            email: None,
            roles: Vec::new(),
            groups: Vec::new(),
            is_authenticated: false,
            organization_id: None,
        }
    }

    pub fn authenticated(user_id: Uuid, email: Option<String>, org_id: Option<Uuid>) -> Self {
        Self {
            user_id,
            email,
            roles: Vec::new(),
            groups: Vec::new(),
            is_authenticated: true,
            organization_id: org_id,
        }
    }

    pub fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.roles = roles;
        self
    }

    pub fn with_groups(mut self, groups: Vec<String>) -> Self {
        self.groups = groups;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionCheckResult {
    pub allowed: bool,
    pub reason: String,
    pub matched_rule: Option<String>,
    pub index_visible: bool,
}

pub struct KbPermissionParser {
    permissions: KbPermissions,
    folder_cache: HashMap<String, ResolvedPermission>,
}

#[derive(Debug, Clone)]
struct ResolvedPermission {
    access: AccessLevel,
    roles: Vec<String>,
    groups: Vec<String>,
    users: Vec<String>,
    index_visibility: AccessLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantPermissionMetadata {
    pub access_level: String,
    pub allowed_roles: Vec<String>,
    pub allowed_groups: Vec<String>,
    pub allowed_users: Vec<String>,
    pub is_public: bool,
    pub requires_auth: bool,
}

#[derive(Debug, Clone)]
pub enum KbPermissionError {
    ParseError(String),
    IoError(String),
    InvalidPath(String),
}

mod parser;
mod metadata;
mod defaults;

pub use defaults::*;
