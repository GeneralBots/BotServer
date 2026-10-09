pub(crate) use crate::api::DirectoryApiState;
pub(crate) use axum::{ extract::{Path, Query, State}, http::StatusCode, response::{Html, IntoResponse}, routing::get, Router, };
pub(crate) use chrono::{DateTime, Utc};
pub(crate) use diesel::RunQueryDsl;
pub(crate) use serde::Deserialize;
pub(crate) use std::sync::Arc;

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub role: Option<String>,
    pub status: Option<String>,
    pub page: Option<u32>,
}

#[derive(Debug, diesel::QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct CountResult {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

pub use part1::*;
use part2::*;
pub use part3::*;
pub use part4::*;
pub use part5::*;
