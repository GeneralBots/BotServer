//! Playlist persistence for the media player.
//!
//! Playlists are stored per branch (org scope) with an optional owner; items
//! reference drive media paths with an explicit order. Playback events are
//! recorded for per-item analytics (plays, watch time, completion).

pub(crate) use axum::{ extract::{Path, Query, State}, http::StatusCode, response::Json, };
pub(crate) use chrono::Utc;
pub(crate) use diesel::prelude::*;
pub(crate) use diesel::result::OptionalExtension;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;
pub(crate) use botcore::shared::state::AppState;

/// Maximum playlist length and item-title length to keep the surface sane.
const MAX_PLAYLIST_ITEMS: usize = 500;
const MAX_TITLE_LEN: usize = 255;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistSummary {
    pub id: Uuid,
    pub name: String,
    pub visibility: String,
    pub item_count: i64,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistItem {
    pub id: Uuid,
    pub media_path: String,
    pub title: String,
    pub position: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistDetail {
    pub id: Uuid,
    pub name: String,
    pub visibility: String,
    pub items: Vec<PlaylistItem>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePlaylistRequest {
    pub name: String,
    #[serde(default = "default_visibility")]
    pub visibility: String,
}

#[derive(Debug, Deserialize)]
pub struct AddItemRequest {
    pub media_path: String,
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct ReorderRequest {
    pub item_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct PlaybackEventRequest {
    pub item_id: Option<Uuid>,
    pub media_path: String,
    #[serde(default = "default_event_type")]
    pub event_type: String,
    pub position_seconds: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct PlaylistQuery {
    pub user_id: Option<Uuid>,
}

#[derive(diesel::QueryableByName)]
struct PlaylistSummaryRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    name: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    visibility: String,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    created_at: chrono::DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    updated_at: chrono::DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Int8)]
    item_count: i64,
}

#[derive(diesel::QueryableByName)]
struct PlaylistItemRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    media_path: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    title: String,
    #[diesel(sql_type = diesel::sql_types::Int4)]
    position: i32,
}

#[derive(diesel::QueryableByName)]
struct CountRow {
    #[diesel(sql_type = diesel::sql_types::Int8)]
    count: i64,
}

#[derive(diesel::QueryableByName)]
struct AnalyticsRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    media_path: String,
    #[diesel(sql_type = diesel::sql_types::Int8)]
    plays: i64,
    #[diesel(sql_type = diesel::sql_types::Int8)]
    completions: i64,
    #[diesel(sql_type = diesel::sql_types::Int8)]
    watch_seconds: i64,
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
use part3::*;
pub use part4::*;
pub use part5::*;
pub use part6::*;
pub use part7::*;
