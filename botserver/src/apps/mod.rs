//! Application catalog and registration.
//!
//! `register` mounts UI-fragment routers and the app catalog endpoint that
//! drives every launcher (start menu, apps menu) in the frontend.

pub mod commands;
pub mod integration_catalog;
pub mod registry;

pub use botuifragments::*;

use axum::routing::{get, post, Router};
use axum::Json;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// Core apps that are always present regardless of feature flags.
const CORE_APPS: &[&str] = &["settings", "auth", "admin"];

/// #1409 — Permanently-visible surface. These apps are NEVER preview and are
/// always enabled, independent of the `.product` file: neither `preview_apps`
/// nor a missing `apps=` entry can hide them. This is the exact set the
/// product owner listed as "must appear always" plus the workbench apps Vibe
/// drives (start menu / sidebar / command palette all resolve them by id).
/// Every other app keeps the preview behaviour (hidden until Preview mode is
/// on). Mirrors `ALWAYS_APP_IDS` in botui/ui/suite/js/window-manager.js.
const ALWAYS_APPS: &[&str] = &[
    "chat",
    "mail",
    "calendar",
    "meet",
    "sheet",
    "people",
    "crm",
    "canvas",
    "player",
    "goals",
    "vibe",
    "integrations",
    "settings",
    "about",
    "browser",
    "database",
    "desktop",
    "plugins",
    "terminal",
    "editor",
];

/// In-memory app-install counters for the App Store (#1156). The frontend
/// overlays these with per-user localStorage state; this endpoint provides a
/// server-side popularity ranking that survives navigation.
#[derive(Default)]
struct AppInstallStats {
    counts: Mutex<HashMap<String, u64>>,
}

static INSTALLS: OnceLock<Arc<AppInstallStats>> = OnceLock::new();

#[derive(serde::Deserialize)]
pub struct InstallBody {
    app_id: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct AgentPlanBody {
    goal: Option<String>,
    app: Option<String>,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

pub use part1::*;
// Explicit import: the `botuifragments::*` glob above also exports a
// `register`, so the app-level one must shadow it by name.
pub use part1::register;
pub use part2::*;
pub use part3::*;
use part4::*;
pub use part5::*;
