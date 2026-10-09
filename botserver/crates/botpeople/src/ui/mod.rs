pub(crate) use axum::{ extract::{Path, Query, State}, response::Html, routing::get, Router, };
pub(crate) use chrono::{DateTime, Utc};
pub(crate) use diesel::prelude::*;
pub(crate) use serde::Deserialize;
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;
pub(crate) use crate::{people, people_departments, people_teams, people_time_off, PeopleState};

#[derive(Queryable)]
struct PersonListRow {
    id: Uuid,
    first_name: String,
    last_name: Option<String>,
    email: Option<String>,
    job_title: Option<String>,
    department: Option<String>,
    avatar_url: Option<String>,
    is_active: bool,
}

#[derive(Queryable)]
struct PersonCardRow {
    id: Uuid,
    first_name: String,
    last_name: Option<String>,
    email: Option<String>,
    job_title: Option<String>,
    department: Option<String>,
    avatar_url: Option<String>,
    phone: Option<String>,
}

#[derive(Queryable)]
struct PersonDetailRow {
    id: Uuid,
    first_name: String,
    last_name: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    mobile: Option<String>,
    job_title: Option<String>,
    department: Option<String>,
    office_location: Option<String>,
    avatar_url: Option<String>,
    bio: Option<String>,
    hire_date: Option<chrono::NaiveDate>,
    is_active: bool,
    last_seen_at: Option<DateTime<Utc>>,
}

#[derive(Queryable)]
struct PersonSearchRow {
    id: Uuid,
    first_name: String,
    last_name: Option<String>,
    email: Option<String>,
    job_title: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct PeopleQuery {
    pub department: Option<String>,
    pub team_id: Option<Uuid>,
    pub is_active: Option<bool>,
    pub search: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

mod part1;
mod part2;
mod part3;

pub use part1::*;
use part2::*;
use part3::*;
