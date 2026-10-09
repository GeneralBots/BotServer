pub(crate) use super::types::*;
pub(crate) use super::utils::*;
pub(crate) use super::validators::validate_basic_code;
pub(crate) use crate::DesignerState;
pub(crate) use axum::{ extract::{Query, State}, response::{Html, IntoResponse}, routing::{get, post}, Json, Router, };
pub(crate) use chrono::Utc;
pub(crate) use diesel::prelude::*;
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;

const ROUTE_FILES: &str = "/api/ui/designer/files";
const ROUTE_LOAD: &str = "/api/ui/designer/load";
const ROUTE_SAVE: &str = "/api/ui/designer/save";
const ROUTE_VALIDATE: &str = "/api/ui/designer/validate";
const ROUTE_EXPORT: &str = "/api/ui/designer/export";
const ROUTE_DIALOGS: &str = "/api/ui/designer/dialogs";
const ROUTE_DIALOG_BY_ID: &str = "/api/ui/designer/dialogs/{id}";
const ROUTE_MODIFY: &str = "/api/ui/designer/modify";
const ROUTE_MAGIC: &str = "/api/ui/designer/magic";
const ROUTE_EDITOR_MAGIC: &str = "/api/ui/editor/magic";

mod part1;
mod part2;

pub use part1::*;
pub use part2::*;
