pub(crate) use axum::{ extract::{Path, State}, response::{Html, IntoResponse}, routing::get, Json, Router, };
pub(crate) use diesel::prelude::*;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;
pub(crate) use crate::TicketsState;

#[derive(Debug, Deserialize)]
pub struct NewCiRequest {
    pub name: String,
    pub ci_type: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NewKbArticleRequest {
    pub title: String,
    pub body: String,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub is_published: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CiRow {
    pub id: Uuid,
    pub name: String,
    pub ci_type: Option<String>,
    pub description: Option<String>,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct KbArticleRow {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub is_published: bool,
}

mod part1;
mod part2;
mod part3;

pub use part1::*;
use part2::*;
pub use part3::*;
