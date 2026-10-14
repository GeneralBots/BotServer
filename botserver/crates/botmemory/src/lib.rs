//! General Bots durable memory service (issue #1178).
//!
//! Owns the `user_memories` table with deduplication by supersession, an
//! LLM-assisted extraction hook fed by chat turns, token-budgeted recall
//! blocks for prompt injection, and JSON import/export over `/api/memory/*`.
//!
//! Distinct from the BASIC `REMEMBER`/`RECALL` keyword shims in
//! `botbasic_ai::keywords`, which persist short-lived session key/value pairs
//! in Redis; this crate stores durable, cross-session semantic memory.

pub mod api;
pub mod extract;
pub mod import;
pub mod models;
pub mod recall;
pub mod schema;
pub mod state;
pub mod store;

use std::sync::Arc;


pub use botlib::db_pool::DbPool;

pub use state::{LlmFn, MemoryService};

/// Builds the memory router over a shared service instance. The integrator
/// merges this router into the server and wires `llm_generate` from the
/// platform LLM provider; providers are never constructed inside this crate.
pub fn configure_routes() -> axum::Router<Arc<MemoryService>> {
    api::configure_routes()
}
