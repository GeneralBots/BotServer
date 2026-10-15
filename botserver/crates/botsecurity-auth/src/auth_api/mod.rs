//! Authentication and authorization API module
//!
//! This module provides a comprehensive authentication and authorization system
//! with support for roles, permissions, bot access control, and multiple
//! authentication methods (API keys, JWT tokens, sessions).

pub mod auth_types_as_alias;
pub mod auth_types_authenticated_user;
pub mod auth_types_from_alias_1;
pub mod auth_types_from_alias_2;
pub mod auth_types_from_alias_3;
pub mod auth_types_from_alias_4;
pub mod auth_types_from_alias_5;
pub mod auth_types_from_alias_6;
pub mod auth_types_from_alias_7;
pub mod auth_types_from_alias_8;
pub mod auth_types_permissions_impl;
pub mod auth_types_users;
pub mod config;
pub mod error;
pub mod middleware;
#[cfg(test)]
pub mod tests;
pub mod types;
// Re-export commonly used types at the module level
pub use config::AuthConfig;
pub use error::AuthError;
pub use middleware::{
    admin_only_middleware, auth_middleware, auth_middleware_with_providers,
    bot_operator_middleware, bot_owner_middleware, bot_scope_middleware,
    require_admin, require_auth_middleware, require_bot_access,
    require_bot_permission, require_permission, require_permission_middleware,
    require_role, require_role_middleware, AuthMiddlewareState,
};
pub use types::{
    AuthenticatedUser, BotAccess, Permission, PublicPathAllowed, Role,
};
pub mod auth_api_auth;
pub mod auth_api_utils;

pub use auth_api_utils::{
    basic_password_as_token, extract_bot_id_from_request, extract_session_from_cookies,
    extract_user_from_request, is_jwt_format, validate_session_sync,
};
pub use auth_api_auth::{
    authenticate_with_extracted_data, extract_user_with_providers,
};
