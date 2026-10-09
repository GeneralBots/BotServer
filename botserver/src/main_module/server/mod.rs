//! HTTP server initialization and routing

pub(crate) use axum::Router;
pub(crate) use std::sync::Arc;
pub(crate) use std::net::SocketAddr;
pub(crate) use log::{error, info, warn};
pub(crate) use tower_http::trace::TraceLayer;
pub(crate) use tower_http::services::ServeDir;
pub(crate) use crate::security::{ create_rate_limit_layer, create_security_headers_layer, request_id_middleware, security_headers_middleware, csrf_middleware, CsrfConfig, CsrfManager, CombinedRateLimiter, HttpRateLimitConfig, PanicHandlerConfig, SecurityHeadersConfig, };
pub(crate) use botcore::shared::state::AppState;
pub(crate) use botlib::SystemLimits;
pub(crate) use super::routes::{security_setup, api_setup, sub_router};

mod part1;
mod part2;

pub use part1::*;
pub(crate) use part2::*;
