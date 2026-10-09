//! Business reports: CRM, billing and support report cards rendered as HTML
//! fragments for the analytics app. All queries are branch-scoped via the
//! configured bot context (global SaaS admin uses `Uuid::nil()`).

pub(crate) use std::sync::Arc;
pub(crate) use axum::{extract::State, response::Html};
pub(crate) use chrono::Utc;
pub(crate) use diesel::RunQueryDsl;
pub(crate) use serde::Deserialize;
pub(crate) use crate::{DbPool, GetDefaultBotFn};

pub struct ReportsState {
    pub pool: Arc<DbPool>,
    pub default_bot: Arc<GetDefaultBotFn>,
}

#[derive(Debug, diesel::QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct CountRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    label: String,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

#[derive(Debug, diesel::QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct ValueRow {
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
    value: Option<bigdecimal::BigDecimal>,
}

#[derive(Debug, diesel::QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct AvgRow {
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Double>)]
    avg: Option<f64>,
}

#[derive(Deserialize)]
pub struct ReportQuery {
    pub q: Option<String>,
}

mod part1;
mod part2;

pub use part1::*;
pub use part2::*;
