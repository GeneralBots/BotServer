//! Split from `api.rs` per #1443 (AGENTS.md 450-line rule).

mod core;
mod metrics;
mod metrics_2;
mod run;
mod tools;
mod tools_2;
#[cfg(test)]
mod tests;

use crate::agent_loop::AgentLoop;
use crate::pipeline::{
    PipelineEngine, PipelineRunContext, PipelineStageReport, RunPipeline, StageStatus,
};
use crate::prompt_manager::VibePromptManager;
use crate::projects::{CreateProjectRequest, ProjectRegistryRef};
use crate::telemetry::VibeTelemetry;
use crate::tool_executor::{ToolDescriptor, VibeToolExecutor};
use crate::types::{VibeProgressEvent, VibeRun, VibeRunConfig, VibeRunState, VibeState, VibeUseCase};
use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use diesel::prelude::*;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use botsecurity_auth::auth_api::types::AuthenticatedUser;

pub use core::{ActionResponse, CancelRunRequest, CapabilitiesResponse, PipelineQuery};
pub(crate) use core::{SiteEnvQuery, parse_site_env_param, parse_use_case};
pub use metrics::{CreateRunResponse, ListRunsQuery, MetricsResponse};
pub(crate) use metrics::{create_run};
pub(crate) use metrics_2::{cancel_run, get_global_metrics, get_run, get_run_metrics, promote_project_site, rollback_project_site, unpublish_project_site};
pub use run::{PipelineResponse};
pub(crate) use run::{MAX_INTENT_CHARS, MAX_TIMEOUT_SECONDS, UnpublishSiteRequest, bot_accessible_to_user, get_pipeline, get_run_events, list_runs, resolve_effective_bot_id, resolve_project, truncate_chars};
pub use tools::{CreateRunRequest, GetRunResponse, ListToolsResponse, VibeApiDeps, VibeSecurityDeps, router};
pub(crate) use tools::{MAX_TOOL_CALLS, VibeApiInner, is_modifying_intent, run_to_response};
pub(crate) use tools_2::{execute_run, list_capabilities_for_use_case, list_tools_for_use_case};
