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

/// Design-time callback that executes a database statement: a schedule
/// insertion, a webhook registration or an event subscription.
pub type DbStatementCallback =
    Box<dyn Fn(&mut PgConnection, &str, &str, Uuid) -> Result<(), String> + Send + Sync>;

/// `USE WEBSITE "<url>"` — records the website-backed knowledge source.
pub type UseWebsiteCallback =
    Box<dyn Fn(&mut PgConnection, &str, Uuid, &str) -> Result<(), String> + Send + Sync>;

/// `tables.bas` — creates or updates the schema the script declares.
pub type TableDefinitionsCallback =
    Box<dyn Fn(Arc<dyn BasicRuntime>, Uuid, &str) -> Result<(), String> + Send + Sync>;

/// Builds the runtime used to execute a compiled script.
pub type RuntimeFactoryCallback =
    Box<dyn Fn(Arc<AppState>) -> Arc<dyn BasicRuntime> + Send + Sync>;

/// `ON UPDATE` — runs after a record changes.
pub type OnUpdateCallback =
    Box<dyn Fn(&mut PgConnection, &str, &str, Uuid, i32) -> Result<(), String> + Send + Sync>;

pub struct CompilerCallbacks {
    pub execute_set_schedule: Option<DbStatementCallback>,
    pub execute_webhook: Option<DbStatementCallback>,
    pub execute_use_website: Option<UseWebsiteCallback>,
    pub process_table_definitions: Option<TableDefinitionsCallback>,
    pub create_runtime: Option<RuntimeFactoryCallback>,
    pub execute_on_update: Option<OnUpdateCallback>,
    /// `ON EVENT "<event>"` — a channel event this tool subscribes to. Fired at
    /// design time: the script names the tool (it is the one being compiled),
    /// so the declaration needs no runtime statement and cannot be skipped by a
    /// tool that only ever runs *because* of the event.
    pub execute_on_event: Option<DbStatementCallback>,
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

mod re_cache;
