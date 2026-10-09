pub(crate) use axum::{ extract::{Path, State}, http::StatusCode, response::{Html, IntoResponse, Json}, Form, Router, };
pub(crate) use diesel::RunQueryDsl;
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;
pub(crate) use botcore::shared::state::AppState;

/// Workspace name of the branch that backs the **default** org on every
/// environment: `{branch}.gbai/` inside the `default.gborg` bucket. Bots
/// created through the suite UI live there (drive_monitor discovers bots
/// from `{branch}.gbai/{bot}.gbdialog/*` object keys).
const DEFAULT_WORKSPACE_PREFIX: &str = "default.gbai";
const DEFAULT_GBORG_BUCKET: &str = "default.gborg";

mod part1;
mod part2;

pub use part1::*;
pub use part2::*;
