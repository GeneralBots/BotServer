//! Vibe subsystem wiring (#743): mounts the botvibe router with a concrete
//! `VibeState` implementation backed by the main AppState pool, and seeds the
//! `vibe_projects` schema at boot.

pub(crate) use botcore::config::ConfigManager;
pub(crate) use botcore::shared::state::AppState;
pub(crate) use botvibe::backups::Backups;
pub(crate) use botvibe::domains::ProjectDomains;
pub(crate) use botvibe::domains_api::domains_router;
pub(crate) use botvibe::members_api::members_router;
pub(crate) use botvibe::metering::VMetering;
pub(crate) use botvibe::metering_api::metering_router;
pub(crate) use botvibe::ops::VmOps;
pub(crate) use botvibe::ops_api::{ops_router, OpsRoutes};
pub(crate) use botvibe::projects::ProjectRegistry;
pub(crate) use botvibe::projects_api::projects_router;
pub(crate) use botvibe::rbac::ProjectRbac;
pub(crate) use botvibe::types::{ LlmConfig, VibeProgressEvent, VibeRun, VibeRunSignal, VibeState, };
pub(crate) use botvibe::vm_lifecycle::VmLifecycle;
pub(crate) use botvibe::vms_api::vms_router;
pub(crate) use botvibe::prompt_manager::VibePromptManager;
pub(crate) use botvibe::telemetry::VibeTelemetry;
pub(crate) use botvibe::tool_executor::{ToolRegistry, VibeToolExecutor};
pub(crate) use log::info;
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::{broadcast, RwLock};
pub(crate) use uuid::Uuid;

struct VibeStateImpl {
    pool: botvibe::types::DbPool,
    progress: Option<broadcast::Sender<VibeProgressEvent>>,
    runs: Arc<RwLock<HashMap<Uuid, VibeRun>>>,
    signals: Option<broadcast::Sender<VibeRunSignal>>,
    config: ConfigManager,
}

mod part1;
mod part2;

pub use part2::*;
