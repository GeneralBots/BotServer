use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub mod alerts;
pub mod scope;
pub mod api;
pub mod api_models;
pub mod billing_admin;
pub mod billing_ui;
pub mod erp_ui;
pub mod invoice;
pub mod lifecycle;
pub mod meters;
pub mod middleware;
pub mod plans;
pub mod quotas;
pub mod schema;
pub mod server_capacity;
pub mod stripe_integration;
pub mod testing;

mod lib_part1;
mod lib_part2;
#[cfg(test)]
mod lib_tests;

pub use lib_part1::*;
pub use lib_part2::*;
