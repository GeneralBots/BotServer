//! Security facade over the four `botsecurity-*` sub-crates.
//!
//! The facade re-exports **module paths**, never flattened item lists: callers
//! spell the namespace they use (`botsecurity::error_sanitizer::log_and_sanitize`,
//! `botsecurity::auth::AuthenticatedUser`) and a rename inside a sub-crate can
//! no longer hide behind a rename alias here (#1500). The sub-crates themselves
//! stay reachable as `botsecurity::botsecurity_core::…` and friends.

pub mod biometric;
pub mod digital_signature;
pub mod integration;
pub mod kyc;
pub mod routes_security;
pub mod schema;

pub use botsecurity_auth;
pub use botsecurity_core;
pub use botsecurity_crypto;
pub use botsecurity_protection;

// ─── botsecurity-core ───
pub use botsecurity_core::{
    antivirus, audit, command_guard, cors, dlp, error_sanitizer, file_validation, headers,
    log_sanitizer, panic_handler, path_guard, prompt_security, request_id, safe_unwrap, scope,
    security_monitoring, sql_guard, tenant, validation, webhook,
};

// ─── botsecurity-auth ───
pub use botsecurity_auth::{
    api_keys, auth, auth_api, auth_provider, blacklist, csrf, jwt, mfa, password, platform_admin,
    rate_limiter, rbac_middleware, request_limits, saas_jwt_auth, session, zitadel_auth,
};
#[cfg(feature = "cache")]
pub use botsecurity_auth::{redis_blacklist_store, redis_csrf_store, redis_session_store};

// ─── botsecurity-crypto ───
pub use botsecurity_crypto::{ca, cert_pinning, encryption, mutual_tls, secrets, tls};

// ─── botsecurity-protection ───
pub use botsecurity_protection::protection;
