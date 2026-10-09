// Server-authoritative operation log with Lamport clocks and version vectors.
//
// This replaces last-write-wins for concurrent collaborative editing. Editors
// submit operations tagged with the version they observed (`base_version`);
// when that base is behind the resource's converged version, the op is a
// concurrent conflict that must be resolved (accept-server / accept-client).
//
// Every op carries an `actor_type` (`human` | `llm`) so a concurrent AI agent's
// edits are ordered, filtered and visualized separately from a person's.

pub(crate) use super::collab_routes::{collab_user_id, collab_user_name, sanitize_resource};
pub(crate) use crate::security::auth_api::types::AuthenticatedUser;
pub(crate) use axum::{ extract::{Path, Query, State}, http::StatusCode, response::Json, routing::{get, post}, Extension, Router, };
pub(crate) use botcore::shared::state::AppState;
pub(crate) use diesel::prelude::*;
pub(crate) use diesel::sql_types::{BigInt, Bool, Text, Timestamptz, Uuid as SqlUuid};
pub(crate) use diesel::PgConnection;
pub(crate) use log::warn;
pub(crate) use serde::Deserialize;
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;

#[derive(Deserialize)]
pub struct SubmitOpBody {
    pub resource_type: String,
    pub resource_id: String,
    pub op_type: String,
    pub base_version: i64,
    #[serde(default)]
    pub actor_type: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Deserialize)]
pub struct OpsQuery {
    pub resource_type: String,
    pub resource_id: String,
    #[serde(default)]
    pub since: Option<i64>,
}

#[derive(Deserialize)]
pub struct ConflictQuery {
    pub resource_type: String,
    pub resource_id: String,
}

#[derive(Deserialize)]
pub struct ResolveBody {
    /// "accept-server" keeps the newer server state; "accept-client" rebases
    /// the conflicting op on top of it.
    pub resolution: String,
}

// ---------------------------------------------------------------------------
// DB helpers (all return diesel::result::Error so they compose in a txn)
// ---------------------------------------------------------------------------

mod part1;
mod part2;
mod part3;

use part1::*;
pub use part2::*;
pub use part3::*;
