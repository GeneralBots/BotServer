pub(crate) use axum::{ extract::State, http::StatusCode, response::{Html, IntoResponse, Json}, routing::{get, post}, Form, Router, };
pub(crate) use chrono::{NaiveDateTime, Utc};
pub(crate) use serde::Deserialize;
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;
pub(crate) use botcore::shared::state::AppState;

#[derive(Debug, Deserialize)]
pub struct SaveEventRequest {
    pub title: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub all_day: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AutotaskRequest {
    pub intent: Option<String>,
}

#[derive(Deserialize)]
pub struct DnsRecordRequest {
    pub hostname: Option<String>,
    pub record_type: Option<String>,
    pub target: Option<String>,
    pub ttl: Option<String>,
}

#[derive(Deserialize)]
pub struct DnsRemoveRequest {
    pub id: Option<String>,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;

use part1::*;
pub use part2::*;
pub use part3::*;
pub use part4::*;
pub use part5::*;
pub use part6::*;
