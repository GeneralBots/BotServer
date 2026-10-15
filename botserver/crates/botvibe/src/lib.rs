//! Vibe project automation.
//!
//! The crate exposes its modules, not a flattened item list (#1500): callers
//! spell the namespace they use (`botvibe::permissions::PermissionEngine`,
//! `botvibe::sessions::SessionRoutes`) so a rename inside a module stays
//! visible at the call site.

pub mod types;
pub mod bootstrap;
pub mod bootstrap_backfill;
pub mod prompt_manager;
pub mod tool_executor;
pub mod telemetry;
pub mod api;
pub mod knowledge_graph;
pub mod agent_loop;
pub mod kiro_bridge;
pub mod projects;
pub mod projects_api;
pub mod project_db;
pub mod vm_lifecycle;
mod vm_incus;
pub mod vms_api;
pub mod publish;
pub mod proxy_sites;
pub mod site_env;
pub mod caddy;
pub mod domains;
pub mod domain_auth;
pub mod domains_api;
pub mod domains_tool;
pub mod eviction;
pub mod harness;
pub mod ops;
pub mod templates;
pub mod scaffold;
pub mod scaffold_locks;
pub mod skills_marketplace;
pub mod skills_store;
pub mod ops_api;
pub mod ops_tools;
pub mod backups;
pub mod rbac;
pub mod members_api;
pub mod metering;
pub mod metering_api;
pub mod metering_schema;
pub mod permissions;
pub mod permissions_api;
pub mod skills;
pub mod schema;
pub mod skill_loader;
pub mod doctor;
pub mod browser;
pub mod canvases;
pub mod catalog_persistence;
pub mod issues;
pub mod websearch;
pub mod gitflow;
pub mod git_mode;
pub mod deploy_source;
pub mod sessions;
pub mod teams;
pub mod capability_registry;
pub mod pipeline;
pub mod run_store;
pub mod grounding;
pub mod wired_tools;
pub mod llm_client;
pub mod daily_briefing;
pub mod planner;
pub mod planner_api;
pub mod moa;
pub mod moa_api;
pub mod agents;
pub mod agents_api;
pub mod browser_memory;
pub mod browser_memory_api;
pub mod browser_driver;
pub mod browser_driver_api;
pub mod proactivity;
pub mod proactivity_api;
pub mod project_locks;

mod re_cache;
