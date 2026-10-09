pub(crate) use axum::{ extract::{Path, State}, response::IntoResponse, };
pub(crate) use diesel::prelude::*;
pub(crate) use log::{info, warn};
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;
pub(crate) use crate::models::{AppState, EmailError, extract_user_from_session};
#[cfg(feature = "mail")]
pub(crate) use crate::models::{EmailSummary, EmailContent};

#[derive(Debug, QueryableByName, serde::Serialize)]
pub struct SignatureRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub name: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub content_html: String,
}

#[derive(Debug, QueryableByName, serde::Serialize)]
pub struct TemplateRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub name: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub subject_template: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub body_html_template: String,
}

#[derive(Debug, QueryableByName, serde::Serialize)]
pub struct RuleRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Varchar)]
    pub name: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub conditions_json: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub actions_json: String,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    pub is_active: bool,
}

use axum::http::StatusCode;

mod part1;
mod part2;
mod part3;
mod part4;

pub use part1::*;
pub use part2::*;
pub use part3::*;
pub use part4::*;
