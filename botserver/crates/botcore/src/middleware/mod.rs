

// ============================================================================
// Organization Context
// ============================================================================

pub(crate) use axum::{ body::Body, extract::{FromRequestParts, State}, http::{header::AUTHORIZATION, request::Parts, Request, StatusCode}, middleware::Next, response::{IntoResponse, Response}, Json, };
pub(crate) use botsecurity_core::response_error::ResponseError;
pub(crate) use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
pub(crate) use serde::{Deserialize, Serialize};
#[cfg(any(feature = "research", feature = "llm"))]
pub(crate) use crate::kb::permissions::{build_qdrant_permission_filter, UserContext};

/// Organization context extracted from request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationContext {
    pub organization_id: Uuid,
    pub organization_name: Option<String>,
    pub plan_id: Option<String>,
    pub is_owner: bool,
    pub permissions: Vec<String>,
}

/// Authenticated user context extracted from request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub roles: Vec<String>,
    pub groups: Vec<String>,
    pub organization_id: Option<Uuid>,
    pub is_admin: bool,
    pub is_super_admin: bool,
    pub token_claims: Option<TokenClaims>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: String,
    pub exp: i64,
    pub iat: i64,
    pub iss: Option<String>,
    pub aud: Option<Vec<String>>,
    pub scope: Option<String>,
    /// Server-minted tenant (`.gborg`) claim. Presence means the token is
    /// bound to an organization; a client-supplied `X-Organization-Id` must
    /// match it or the request is rejected (issue #736).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_id: Option<String>,
    /// Server-minted workspace branch claim, used to scope CRM and app data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
}

// ============================================================================
// Request Context (Combined)
// ============================================================================

/// Combined request context with organization and user information
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub user: AuthenticatedUser,
    pub organization: Option<OrganizationContext>,
    pub request_id: Uuid,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub bot_id: Option<Uuid>,
    pub conversation_id: Option<Uuid>,
}

#[derive(Clone)]
pub struct ContextMiddlewareState {
    pub jwt_secret: Arc<String>,
    pub org_cache: Arc<RwLock<std::collections::HashMap<Uuid, CachedOrganization>>>,
    pub user_cache: Arc<RwLock<std::collections::HashMap<Uuid, CachedUserData>>>,
    pub cache_ttl_seconds: u64,
}

#[derive(Clone)]
pub struct CachedOrganization {
    pub context: OrganizationContext,
    pub cached_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone)]
pub struct CachedUserData {
    pub roles: Vec<String>,
    pub groups: Vec<String>,
    pub cached_at: chrono::DateTime<chrono::Utc>,
}

type MiddlewareFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, ResponseError>> + Send>>;

#[derive(Debug)]
enum AuthError {
    MissingToken,
    InvalidFormat,
    InvalidToken(String),
    TokenExpired,
}

struct UnauthorizedResponse {
    message: String,
}

struct ForbiddenResponse {
    message: String,
}

struct BadRequestResponse {
    message: String,
}

/// Standard organization roles
pub struct OrgRoles;

/// Standard permissions
pub struct Permissions;

use std::sync::Arc;
use tokio::sync::RwLock;
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

pub use part4::*;
pub use part5::*;
pub use part9::*;
pub use part11::*;
