//! Split from `publish.rs` per #1443 (AGENTS.md 450-line rule).
//! #757 — `publish_project` Vibe tool.
//!
//! Puts a project onto a live environment: ensures the target env VM
//! (#744), calls the deployment API (`/api/deployment/deploy`) so an Incus
//! container (or Caddy route) is raised, optionally binds a custom domain,
//! and records the deployment in the project payload (deployment history —
//! #772 reads the same records).

mod collect;
mod site;
mod site_2;
mod site_3;
#[cfg(test)]
mod tests;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use axum::extract::{Extension, Path as AxumPath};
use axum::http::{header, StatusCode};
use axum::routing::get;
use axum::Router;
use diesel::RunQueryDsl;
use serde_json::Value;
use uuid::Uuid;
use crate::types::DbPool;
use crate::domains::{BindDomainRequest, ProjectDomains};
use crate::harness;
use crate::projects::{Project, ProjectRegistry};
use crate::tool_executor::{ToolHandler, ToolSchema};
use crate::types::{VibeState, VibeToolResult, VibeUseCase};
use crate::vm_lifecycle::{CreateVmRequest, VmLifecycle};

pub use collect::{publish_project_tool};
pub(crate) use collect::{PUBLISH_DEFAULT_ENV, publish_max_bytes_budget, walk_workspace};
#[cfg(not(target_os = "windows"))]
pub(crate) use collect::{api_base};
pub use site::{PUBLISH_PRODUCTION_STAMP, publish_project_schema, publish_router, published_domain};
pub(crate) use site::{collect_workspace_files, production_approved};
pub(crate) use site_2::{do_publish};
#[cfg(not(target_os = "windows"))]
pub(crate) use site_3::start_prod_app;
pub(crate) use site_3::SitePublish;
