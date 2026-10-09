pub(crate) use axum::{ extract::State, http::{header, StatusCode}, response::{IntoResponse, Json}, routing::{get, post}, Router, };
pub(crate) use log::{debug, info, warn};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use botcore::shared::utils::get_stack_path;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use once_cell::sync::Lazy;
pub(crate) use std::sync::OnceLock;
pub(crate) use botcore::shared::state::AppState;
pub(crate) use crate::DbPool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUserData {
    pub user_id: String,
    pub email: String,
    pub username: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub display_name: Option<String>,
    pub organization_id: Option<String>,
    pub roles: Vec<String>,
    pub bucket: Option<String>,
    pub created_at: i64,
}

pub static SESSION_CACHE: Lazy<RwLock<HashMap<String, SessionUserData>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Session lifetime in seconds. Mirrors `expires_in` returned by login;
/// enforced on every `get_current_user` lookup (cache + persisted store) so
/// stale sessions cannot outlive their TTL after a restart.
const SESSION_TTL_SECS: i64 = 3600;

/// Optional DB pool used to persist suite sessions across restarts. Wired once
/// at bootstrap (main.rs); when present, session creation writes a row to
/// `login_sessions` and logout deletes it, and the auth lookup rehydrates on
/// in-memory cache misses.
static SESSION_POOL: OnceLock<DbPool> = OnceLock::new();

/// #1364 — pluggable verifier for token kinds minted OUTSIDE this crate
/// (e.g. the cloud management JWT signed with the SaaS secret). Wired once at
/// bootstrap by the owning module; when it returns `Some(user)`,
/// `get_current_user` treats the bearer as an authenticated identity instead
/// of falling through to the anonymous response (sidebar showed "Sign in"
/// while the user was actually signed in on the cloud/SaaS surface).
static EXTERNAL_TOKEN_VERIFIER: OnceLock<fn(&str) -> Option<SessionUserData>> = OnceLock::new();

const BOOTSTRAP_SECRET_ENV: &str = "GB_BOOTSTRAP_SECRET";

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub remember: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_in: Option<i64>,
    pub requires_2fa: bool,
    pub session_token: Option<String>,
    pub redirect: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CurrentUserResponse {
    pub id: Option<String>,
    pub username: Option<String>,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub display_name: Option<String>,
    pub roles: Option<Vec<String>>,
    pub organization_id: Option<String>,
    pub bucket: Option<String>,
    pub avatar_url: Option<String>,
    pub is_anonymous: bool,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct TwoFactorRequest {
    pub session_token: String,
    pub code: String,
    pub trust_device: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct BootstrapAdminRequest {
    pub bootstrap_secret: String,
    pub email: String,
    pub username: String,
    pub password: String,
    pub first_name: String,
    pub last_name: String,
    pub organization_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BootstrapResponse {
    pub success: bool,
    pub message: String,
    pub user_id: Option<String>,
    pub organization_id: Option<String>,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;
mod part8;

pub use part1::*;
pub use part2::*;
pub use part3::*;
pub use part4::*;
pub use part5::*;
pub use part6::*;
pub use part7::*;
use part8::*;
