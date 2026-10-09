use botcore::shared::state::AppState;
use botcore::shared::utils::current_org_id;
use botbasic_types::BasicRuntime;
use diesel::PgConnection;
use std::sync::Arc;
use std::fmt;
use uuid::Uuid;

pub mod basic_errors;
pub mod blocks;
pub mod goto_transform;
pub mod save_conversion;
pub mod syntax_transforms;
pub mod tool_parsing;
pub mod types;

pub use types::*;

pub struct CompilerCallbacks {
    pub execute_set_schedule: Option<Box<dyn Fn(&mut PgConnection, &str, &str, Uuid) -> Result<(), String> + Send + Sync>>,
    pub execute_webhook: Option<Box<dyn Fn(&mut PgConnection, &str, &str, Uuid) -> Result<(), String> + Send + Sync>>,
    pub execute_use_website: Option<Box<dyn Fn(&mut PgConnection, &str, Uuid, &str) -> Result<(), String> + Send + Sync>>,
    pub process_table_definitions: Option<Box<dyn Fn(Arc<dyn BasicRuntime>, Uuid, &str) -> Result<(), String> + Send + Sync>>,
    pub create_runtime: Option<Box<dyn Fn(Arc<AppState>) -> Arc<dyn BasicRuntime> + Send + Sync>>,
    pub execute_on_update: Option<Box<dyn Fn(&mut PgConnection, &str, &str, Uuid, i32) -> Result<(), String> + Send + Sync>>,
    /// `ON EVENT "<event>"` — a channel event this tool subscribes to. Fired at
    /// design time: the script names the tool (it is the one being compiled),
    /// so the declaration needs no runtime statement and cannot be skipped by a
    /// tool that only ever runs *because* of the event.
    pub execute_on_event: Option<Box<dyn Fn(&mut PgConnection, &str, &str, Uuid) -> Result<(), String> + Send + Sync>>,
}

#[derive(Debug)]
pub struct BasicCompiler {
    state: Arc<AppState>,
    bot_id: Uuid,
    previous_schedules: std::collections::HashSet<String>,
    pub callbacks: CompilerCallbacks,
}

mod lib_part1;
mod lib_part2;
mod lib_part3;
