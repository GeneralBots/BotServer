


// ---------------------------------------------------------------------------
// Protocol helpers
// ---------------------------------------------------------------------------

pub(crate) use axum::extract::{ConnectInfo, Extension, State, WebSocketUpgrade};
pub(crate) use axum::http::StatusCode;
pub(crate) use axum::response::IntoResponse;
pub(crate) use axum::Json;
pub(crate) use diesel::prelude::*;
pub(crate) use serde::Deserialize;
pub(crate) use tracing::{debug, info, warn};
pub(crate) use uuid::Uuid;
pub(crate) use botsecurity_auth::auth_api::types::AuthenticatedUser;
pub(crate) use crate::models::{ConnectionConfig, DesktopSession, SessionAuditEvent};
pub(crate) use crate::proxy::{extract_client_ip, handle_ws_connection, rdp_health_check, tcp_health_check};
pub(crate) use crate::session_manager::{SessionError, SessionManager};
pub(crate) use crate::types::{ConnectionCreateRequest, HealthCheckResponse, mask_ip};

/// Remote-desktop protocols the proxy tunnel may carry.
const SUPPORTED_PROTOCOLS: &[&str] = &["vnc", "rdp"];

pub type DbPool = diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>;

// ---------------------------------------------------------------------------
// Shared application state
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AppState {
    pub session_manager: SessionManager,
    pub config: ConnectionConfig,
    pub pool: Option<DbPool>,
}

// ---------------------------------------------------------------------------
// POST /connections — create & register a proxy session
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ConnectRequest {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub protocol: Option<String>,
    pub auth_type: Option<String>,
    /// Optional RDP target password, vaulted server-side (never persisted).
    #[serde(default)]
    pub password: Option<String>,
    /// Optional RDP domain for NLA authentication.
    #[serde(default)]
    pub domain: Option<String>,
}

const PREFIX: &str = "/api/desktop";

use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_builds() {
        let state = AppState {
            session_manager: SessionManager::new(ConnectionConfig::default()),
            config: ConnectionConfig::default(),
            pool: None,
        };
        let _app = router(state);
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;

use part1::*;
pub use part2::*;
pub use part3::*;
pub use part4::*;
pub use part5::*;
