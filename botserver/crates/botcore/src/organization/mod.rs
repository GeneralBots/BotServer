//! Organization Management Module
//!
//! Provides organization creation, role management, group management,
//! and access control for multi-tenant deployments.



// ============================================================================
// Organization Types
// ============================================================================

pub(crate) use chrono::{DateTime, Utc};
pub(crate) use serde::{Deserialize, Serialize};

/// Organization entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub website: Option<String>,
    pub plan_id: String,
    pub owner_id: Uuid,
    pub settings: OrganizationSettings,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Organization settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationSettings {
    pub allow_public_bots: bool,
    pub require_2fa: bool,
    pub allowed_email_domains: Vec<String>,
    pub default_user_role: String,
    pub max_members: Option<u32>,
    pub sso_enabled: bool,
    pub sso_provider: Option<String>,
    pub audit_log_retention_days: u32,
    pub ip_safelist: Vec<String>,
    pub custom_branding: Option<CustomBranding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomBranding {
    pub primary_color: String,
    pub secondary_color: Option<String>,
    pub logo_url: Option<String>,
    pub favicon_url: Option<String>,
    pub custom_css: Option<String>,
}

// ============================================================================
// Organization Member
// ============================================================================

/// Organization member entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationMember {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub status: MemberStatus,
    pub invited_by: Option<Uuid>,
    pub invited_at: Option<DateTime<Utc>>,
    pub joined_at: Option<DateTime<Utc>>,
    pub last_active_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberStatus {
    Active,
    Invited,
    Suspended,
    Deactivated,
}

// ============================================================================
// Roles
// ============================================================================

/// Role definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: Uuid,
    pub organization_id: Option<Uuid>, // None for system roles
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub is_system: bool,
    pub is_default: bool,
    pub hierarchy_level: u32, // Lower = more powerful
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Group definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    pub parent_group_id: Option<Uuid>,
    pub is_system: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Group member
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    pub id: Uuid,
    pub group_id: Uuid,
    pub user_id: Uuid,
    pub added_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// User role within an organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRole {
    pub id: Uuid,
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub role_id: Uuid,
    pub role_name: String,
    pub assigned_by: Option<Uuid>,
    pub assigned_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Bot access configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotAccessConfig {
    pub bot_id: Uuid,
    pub organization_id: Uuid,
    pub visibility: BotVisibility,
    pub allowed_roles: Vec<String>,
    pub allowed_groups: Vec<String>,
    pub allowed_users: Vec<Uuid>,
    pub denied_users: Vec<Uuid>,
    pub requires_authentication: bool,
    pub ip_restrictions: Vec<String>,
    pub rate_limit_per_user: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BotVisibility {
    Private,      // Only explicit users/roles/groups
    Organization, // All org members
    Public,       // Anyone (optionally with auth)
}

// ============================================================================
// App Access Control
// ============================================================================

/// App access configuration (Forms, Sites, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppAccessConfig {
    pub app_id: Uuid,
    pub app_type: AppType,
    pub organization_id: Uuid,
    pub visibility: AppVisibility,
    pub allowed_roles: Vec<String>,
    pub allowed_groups: Vec<String>,
    pub allowed_users: Vec<Uuid>,
    pub requires_authentication: bool,
    pub submission_requires_auth: bool, // For forms
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppType {
    Form,
    Site,
    Dashboard,
    Report,
    Workflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppVisibility {
    Private,
    Organization,
    Public,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppAction {
    View,
    Edit,
    Submit,
    Admin,
}

// ============================================================================
// Access Check Types
// ============================================================================

/// User context for access checks
#[derive(Debug, Clone)]
pub struct UserAccessContext {
    pub user_id: Uuid,
    pub is_authenticated: bool,
    pub organization_id: Option<Uuid>,
    pub roles: Vec<String>,
    pub groups: Vec<String>,
    pub permissions: Vec<String>,
}

/// Access check result
#[derive(Debug, Clone)]
pub enum AccessCheckResult {
    Allowed,
    Denied(String),
    RequiresElevation(String),
}

/// Organization management service
pub struct OrganizationService {
    /// Database connection pool for organization operations
    _db_pool: DbPool,
}

/// Result of organization creation
#[derive(Debug)]
pub struct OrganizationCreationResult {
    pub organization: Organization,
    pub roles: Vec<Role>,
    pub groups: Vec<Group>,
    pub owner_member: OrganizationMember,
    pub owner_role: UserRole,
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, Clone)]
pub enum OrganizationError {
    NotFound,
    AlreadyExists,
    InvalidName(String),
    PermissionDenied(String),
    MemberLimitReached,
    InvalidRole(String),
    InvalidGroup(String),
    DatabaseError(String),
}

pub mod permissions {
    pub const ORG_MANAGE: &str = "org:manage";
    pub const ORG_BILLING: &str = "org:billing";
    pub const ORG_MEMBERS: &str = "org:members";
    pub const ORG_MEMBERS_VIEW: &str = "org:members:view";
    pub const ORG_SETTINGS: &str = "org:settings";
    pub const ORG_DELETE: &str = "org:delete";

    pub const BOT_CREATE: &str = "bot:create";
    pub const BOT_VIEW: &str = "bot:view";
    pub const BOT_EDIT: &str = "bot:edit";
    pub const BOT_DELETE: &str = "bot:delete";
    pub const BOT_PUBLISH: &str = "bot:publish";
    pub const BOT_ALL: &str = "bot:*";

    pub const KB_READ: &str = "kb:read";
    pub const KB_WRITE: &str = "kb:write";
    pub const KB_ADMIN: &str = "kb:admin";
    pub const KB_ALL: &str = "kb:*";

    pub const APP_CREATE: &str = "app:create";
    pub const APP_VIEW: &str = "app:view";
    pub const APP_EDIT: &str = "app:edit";
    pub const APP_DELETE: &str = "app:delete";
    pub const APP_ADMIN: &str = "app:admin";
    pub const APP_ALL: &str = "app:*";

    pub const ANALYTICS_VIEW: &str = "analytics:view";
    pub const ANALYTICS_EXPORT: &str = "analytics:export";
    pub const ANALYTICS_ALL: &str = "analytics:*";

    pub const WILDCARD: &str = "*";
}

use std::collections::HashMap;
use uuid::Uuid;
use crate::shared::utils::DbPool;

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;
mod part8;
mod part9;
mod part10;
mod part11;
mod part12;
mod part13;

pub use part4::*;
pub use part6::*;
use part13::*;
