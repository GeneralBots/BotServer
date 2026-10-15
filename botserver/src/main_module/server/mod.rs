//! HTTP server initialization and routing

pub(crate) use axum::Router;
pub(crate) use std::sync::Arc;
pub(crate) use std::net::SocketAddr;
pub(crate) use log::{error, info, warn};
pub(crate) use tower_http::trace::TraceLayer;
pub(crate) use tower_http::services::ServeDir;
pub(crate) use crate::security::csrf::{csrf_middleware, CsrfConfig, CsrfManager};
pub(crate) use crate::security::headers::{
    create_security_headers_layer, security_headers_middleware, SecurityHeadersConfig,
};
pub(crate) use crate::security::panic_handler::PanicHandlerConfig;
pub(crate) use crate::security::rate_limiter::{
    create_rate_limit_layer, CombinedRateLimiter, HttpRateLimitConfig,
};
pub(crate) use crate::security::request_id::request_id_middleware;
pub(crate) use botcore::shared::state::AppState;
pub(crate) use botlib::limits::SystemLimits;
pub(crate) use super::routes::{security_setup, api_setup, sub_router};

mod part1;
mod part2;

pub use part1::*;
pub(crate) use part2::*;
