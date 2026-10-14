//! General Bots per-app agent consent system (issue #1176).
//!
//! Executors call [`authorize`] before performing an application action; a
//! missing grant produces a five-minute [`PendingRequest`] rendered as an
//! HTMX consent card. Users resolve prompts through `/api/consent/resolve`;
//! `allow_once` payloads are handed back to executors via
//! [`take_pending_approved`]. Sensitive action classes (`payment`) ignore
//! stored "always" grants once the monthly cycle ends.

pub mod api;
pub mod cards;
pub mod enforce;
pub mod models;
pub mod schema;
pub mod store;


pub use botlib::db_pool::DbPool;

pub use enforce::{
    authorize, resolve, take_pending_approved, ConsentDecision, ConsentService, PendingRequest,
    ResolvedOutcome,
};
pub use models::{AppPermissionRow, Decision, GrantBody, ResolveBody};

/// Builds the consent router over a shared service instance. The integrator
/// merges this into the server router and calls
/// `ConsentService::ensure_sweeper` once at boot.
use std::sync::Arc;

pub fn configure_routes() -> axum::Router<Arc<ConsentService>> {
    api::configure_routes()
}
