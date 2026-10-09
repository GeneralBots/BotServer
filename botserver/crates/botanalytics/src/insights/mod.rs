pub(crate) use axum::{ extract::{Query, State}, routing::{get, post, put}, Json, Router, };
pub(crate) use chrono::{Datelike, Duration, NaiveDate, Utc};
pub(crate) use diesel::prelude::*;
pub(crate) use diesel::sql_types::{BigInt, Date, Double, Uuid as DieselUuid};
pub(crate) use log::debug;
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;
pub(crate) use crate::insights_types::*;
pub(crate) use crate::DbPool;

#[derive(Debug, Clone)]
pub struct InsightsService {
    pool: DbPool,
}

#[derive(diesel::QueryableByName)]
struct DailyInsightsRow {
    #[diesel(sql_type = DieselUuid)]
    id: Uuid,
    #[diesel(sql_type = DieselUuid)]
    user_id: Uuid,
    #[diesel(sql_type = Date)]
    date: NaiveDate,
    #[diesel(sql_type = BigInt)]
    total_active_time: i64,
    #[diesel(sql_type = BigInt)]
    focus_time: i64,
    #[diesel(sql_type = BigInt)]
    meeting_time: i64,
    #[diesel(sql_type = BigInt)]
    email_time: i64,
    #[diesel(sql_type = BigInt)]
    chat_time: i64,
    #[diesel(sql_type = BigInt)]
    document_time: i64,
    #[diesel(sql_type = Double)]
    collaboration_score: f64,
    #[diesel(sql_type = Double)]
    wellbeing_score: f64,
    #[diesel(sql_type = Double)]
    productivity_score: f64,
}

#[derive(diesel::QueryableByName)]
struct ActivityAggRow {
    #[diesel(sql_type = Date)]
    activity_date: NaiveDate,
    #[diesel(sql_type = BigInt)]
    focus_time: i64,
    #[diesel(sql_type = BigInt)]
    meeting_time: i64,
    #[diesel(sql_type = BigInt)]
    email_time: i64,
    #[diesel(sql_type = BigInt)]
    chat_time: i64,
    #[diesel(sql_type = BigInt)]
    document_time: i64,
    #[diesel(sql_type = BigInt)]
    total_time: i64,
}

#[derive(diesel::QueryableByName)]
struct ActivityStatsRow {
    #[diesel(sql_type = diesel::sql_types::Nullable<Double>)]
    avg_focus: Option<f64>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Double>)]
    avg_meeting: Option<f64>,
    #[diesel(sql_type = diesel::sql_types::Nullable<Double>)]
    avg_active: Option<f64>,
    #[diesel(sql_type = BigInt)]
    _days_tracked: i64,
}

mod part1;
mod part2;
mod part3;

pub use part3::*;
