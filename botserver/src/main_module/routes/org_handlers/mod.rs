pub(crate) use axum::{Json, extract::State};
pub(crate) use std::sync::Arc;
pub(crate) use log::{info, warn};
pub(crate) use botcore::shared::state::AppState;
pub(crate) use diesel::prelude::*;
pub(crate) use diesel::ExpressionMethods;
pub(crate) use diesel::QueryDsl;

#[derive(serde::Deserialize)]
pub struct Office365MigrationRequest {
    pub tenant_id: String,
    pub client_id: String,
    pub client_secret: String,
    pub sync_mode: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct CreateOrgRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
}

mod part1;
mod part2;
mod part3;

pub use part1::*;
pub use part2::*;
pub use part3::*;
