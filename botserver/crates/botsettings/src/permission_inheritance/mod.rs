pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::{HashMap, HashSet};
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Permission {
    pub id: Uuid,
    pub name: String,
    pub resource: String,
    pub action: PermissionAction,
    pub scope: PermissionScope,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PermissionAction {
    Create,
    Read,
    Update,
    Delete,
    Execute,
    Admin,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionScope {
    Global,
    Organization,
    Bot,
    App,
    Resource(String),
}

#[derive(Debug, Clone)]
pub struct RoleHierarchy {
    roles: HashMap<String, RoleNode>,
    permission_cache: HashMap<String, HashSet<String>>,
}

#[derive(Debug, Clone)]
struct RoleNode {
    permissions: HashSet<String>,
    parent_roles: Vec<String>,
    hierarchy_level: i32,
}

#[derive(Debug, Clone)]
pub struct GroupHierarchy {
    groups: HashMap<String, GroupNode>,
    permission_cache: HashMap<String, HashSet<String>>,
}

#[derive(Debug, Clone)]
struct GroupNode {
    permissions: HashSet<String>,
    parent_group: Option<String>,
    child_groups: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PermissionInheritanceResolver {
    role_hierarchy: RoleHierarchy,
    group_hierarchy: GroupHierarchy,
    user_roles: HashMap<Uuid, HashSet<String>>,
    user_groups: HashMap<Uuid, HashSet<String>>,
    user_direct_permissions: HashMap<Uuid, HashSet<String>>,
}

#[derive(Debug, Clone)]
pub struct EffectivePermissions {
    pub user_id: Uuid,
    pub permissions: HashSet<String>,
    pub sources: Vec<PermissionSource>,
}

#[derive(Debug, Clone)]
pub struct PermissionSource {
    pub permission: String,
    pub source_type: PermissionSourceType,
    pub source_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionSourceType {
    Direct,
    Role,
    Group,
}

#[derive(Debug, Clone)]
pub struct PermissionCheckResult {
    pub allowed: bool,
    pub permission: String,
    pub source: Option<PermissionSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePermission {
    pub resource_type: String,
    pub resource_id: Uuid,
    pub organization_id: Uuid,
    pub owner_id: Uuid,
    pub visibility: ResourceVisibility,
    pub allowed_roles: Vec<String>,
    pub allowed_groups: Vec<String>,
    pub allowed_users: Vec<Uuid>,
    pub denied_users: Vec<Uuid>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceVisibility {
    Private,
    Organization,
    Public,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

pub use part4::*;
