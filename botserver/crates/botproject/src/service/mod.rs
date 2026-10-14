pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use uuid::Uuid;
pub(crate) use chrono::Utc;
pub(crate) use diesel::prelude::*;
pub(crate) use log::warn;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use crate::import::ImportResult;
pub(crate) use crate::types::*;

/// Postgres pool shared by every persistence operation. Kept as a concrete
/// diesel type (not botcore's alias) so this leaf crate stays decoupled.
pub use botlib::db_pool::DbPool;

/// Full tenant snapshot: one org's projects and everything hanging off them.
/// Serialized to a single JSONB row keyed by org_id in `project_snapshots`.
#[derive(Serialize, Deserialize, Default)]
struct OrgSnapshot {
    projects: Vec<Project>,
    tasks: Vec<ProjectTask>,
    resources: Vec<Resource>,
    assignments: Vec<ResourceAssignment>,
}

pub struct ProjectService {
    pub(crate) projects: Arc<RwLock<HashMap<Uuid, Project>>>,
    pub(crate) tasks: Arc<RwLock<HashMap<Uuid, ProjectTask>>>,
    pub(crate) resources: Arc<RwLock<HashMap<Uuid, Resource>>>,
    pub(crate) assignments: Arc<RwLock<HashMap<Uuid, ResourceAssignment>>>,
    pool: Option<DbPool>,
}

    /// In-memory only (used where no pool is available, e.g. the AppState
    /// placeholder). Prefer [`ProjectService::with_pool`] in the router.

mod part1;
mod part2;
mod part3;
