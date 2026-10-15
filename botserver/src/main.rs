

#![recursion_limit = "512"]

use uuid::Uuid;

// Module declarations
pub mod main_module; // ci-timing
pub mod session_pool;
pub mod soon_features;

// Re-export commonly used items from main_module
pub use main_module::{BootstrapProgress, health_check, health_check_simple, receive_client_errors};

// Use mimalloc as the global allocator when the feature is enabled (replaced tikv-jemalloc due to RUSTSEC-2024-0436)
#[cfg(feature = "jemalloc")]
use mimalloc::MiMalloc;

#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

// Module declarations for feature-gated modules
#[cfg(feature = "analytics")]
pub mod analytics;
#[cfg(feature = "attendant")]
pub mod attendant;
#[cfg(feature = "automation")]
pub use botautotask as auto_task;
pub mod autotask_host;
#[cfg(feature = "vibe")]
pub mod vibe;
#[cfg(feature = "scripting")]
pub mod basic;
#[cfg(feature = "billing")]
pub mod billing;
#[cfg(feature = "saas")]
pub mod management;
pub mod botmodels;
pub mod jukebox;
#[cfg(feature = "canvas")]
pub mod canvas;
#[cfg(feature = "social")]
pub mod channels;
#[cfg(feature = "people")]
pub mod contacts;
#[cfg(feature = "people")]
pub mod crm;
pub mod core;
#[cfg(feature = "designer")]
pub mod designer;
#[cfg(feature = "dashboards")]
pub mod dashboards;
#[cfg(feature = "deployment")]
pub mod deployment;
pub mod api;
pub mod browser;
#[cfg(feature = "docs")]
pub mod docs;
pub mod embedded_ui;
#[cfg(feature = "learn")]
pub mod learn;
#[cfg(feature = "legal")]
pub mod legal;
#[cfg(feature = "maintenance")]
pub mod maintenance;
#[cfg(feature = "monitoring")]
pub mod monitoring;
#[cfg(feature = "multimodal")]
pub mod multimodal;
#[cfg(feature = "marketing")]
pub mod marketing;
#[cfg(feature = "paper")]
pub mod paper;
#[cfg(feature = "people")]
pub mod people;
#[cfg(feature = "player")]
pub mod player;
#[cfg(feature = "products")]
pub mod products;
#[cfg(feature = "project")]
pub mod project;
#[cfg(feature = "research")]
pub mod research;
#[cfg(feature = "search")]
pub mod search;
#[cfg(feature = "security")]
pub mod security;
pub mod settings;
#[cfg(feature = "sheet")]
pub mod sheet;
#[cfg(feature = "slides")]
pub mod slides;
#[cfg(feature = "plan")]
pub mod plan;
#[cfg(feature = "social")]
pub mod social;
#[cfg(feature = "sources")]
pub mod sources;
#[cfg(feature = "tickets")]
pub mod tickets;
#[cfg(feature = "video")]
pub mod video;
#[cfg(feature = "workspaces")]
pub mod workspaces;
pub mod apps;


#[cfg(feature = "desktop")]
pub mod desktop;

#[cfg(feature = "fraud")]
pub mod fraud;

#[cfg(feature = "inventory")]
pub mod inventory;

#[cfg(feature = "gl")]
pub mod gl;

#[cfg(feature = "retail")]
pub mod retail;

#[cfg(feature = "attendant")]
pub mod attendance;

#[cfg(feature = "calendar")]
pub mod calendar;

#[cfg(feature = "compliance")]
pub mod compliance;

#[cfg(feature = "console")]
pub mod console;

#[cfg(feature = "directory")]
pub mod directory;

#[cfg(feature = "drive")]
pub mod drive;

#[cfg(feature = "mail")]
pub mod email;

#[cfg(feature = "instagram")]
pub mod instagram;

#[cfg(feature = "llm")]
pub mod llm;

#[cfg(feature = "meet")]
pub mod meet;

#[cfg(feature = "msteams")]
pub mod msteams;

#[cfg(feature = "nvidia")]
pub mod nvidia;

#[cfg(feature = "tasks")]
pub mod tasks;

#[cfg(feature = "timeseries")]
pub mod timeseries;

#[cfg(feature = "vectordb")]
#[path = "vector-db/mod.rs"]
pub mod vector_db;

#[cfg(feature = "weba")]
pub mod weba;

#[cfg(feature = "whatsapp")]
pub mod whatsapp;


#[cfg(feature = "telegram")]
pub mod telegram;

// Re-export commonly used types
#[cfg(feature = "llm")]
pub use llm::cache::{CacheConfig, CachedLLMProvider, CachedResponse, LocalEmbeddingService};
#[cfg(feature = "llm")]
pub use llm::DynamicLLMProvider;

#[cfg(feature = "tasks")]
pub use tasks::TaskEngine;

use dotenvy::dotenv;
use log::{error, info, trace, warn};

use std::sync::Arc;
#[cfg(feature = "security")]
use crate::security::panic_handler::set_global_panic_hook;

mod main_body;

/// Crate entry point. The boot sequence lives in [`main_body`] (split from
/// this file for the 450-line limit) and carries `#[tokio::main]`, so this
/// wrapper only forwards the exit status.
fn main() -> std::io::Result<()> {
    main_body::main()
}
