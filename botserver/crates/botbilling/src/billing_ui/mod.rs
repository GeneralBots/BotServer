pub(crate) use axum::{ extract::{Query, State}, http::{HeaderMap, StatusCode}, response::{Html, IntoResponse}, routing::{get, post, put}, Json, Router, };
pub(crate) use bigdecimal::{BigDecimal, ToPrimitive};
pub(crate) use chrono::{DateTime, Datelike, NaiveDate, Utc};
pub(crate) use diesel::prelude::*;
pub(crate) use serde::Deserialize;
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;
pub(crate) use crate::api::BillingApiState;
pub(crate) use crate::schema::{billing_invoices, billing_payments, billing_quotes, billing_recurring};

#[derive(diesel::QueryableByName)]
struct CountRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

const DEFAULT_QUOTA_LIMITS: &[(&str, i64)] = &[
    ("invoices", 100),
    ("team", 50),
    ("bots", 10),
];

#[derive(Debug, Deserialize)]
pub struct StatusQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

#[derive(Deserialize)]
struct UpgradeRequest {
    plan_id: String,
}

#[derive(Deserialize)]
struct CancelRequest {
    reason: Option<String>,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;

use part1::*;
use part2::*;
pub use part3::*;
use part4::*;
use part5::*;
use part6::*;
use part7::*;
