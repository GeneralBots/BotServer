//! Hardened agentic browser control plane (#1175 backend).
//!
//! Owns browsing policy enforcement (`policy`), step/cost budgets
//! (`budget`), task lifecycle orchestration (`tasks`) and the cross-session
//! browsing memory (`memory`). The actual page-driving execution is performed
//! by an external driver service that reports steps through
//! `POST /api/browser/tasks/:id/advance`; policy and budget validation happen
//! before any step is recorded.

pub mod api;
pub mod budget;
pub mod memory;
pub mod models;
pub mod policy;
pub mod schema;
pub mod tasks;

use axum::Router;
use std::sync::Arc;

/// Shared PostgreSQL connection pool (same shape as sibling crates).
pub use botlib::db_pool::DbPool;

pub struct BrowserPolicyService {
    pool: DbPool,
}

impl BrowserPolicyService {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &DbPool {
        &self.pool
    }
}

/// Router fragment merged by the integrator under the authenticated scope.
pub fn configure_routes() -> Router<Arc<BrowserPolicyService>> {
    api::configure_routes()
}
