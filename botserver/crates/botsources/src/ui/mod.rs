pub mod mcp_add;
pub mod mcp_catalog;

use crate::state::AppState;
use axum::{extract::State, response::Html, routing::get, Router};
use std::sync::Arc;

mod part1;
mod part2;

pub use part1::*;
pub use part2::*;
