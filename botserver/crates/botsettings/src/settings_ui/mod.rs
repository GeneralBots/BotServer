//! Settings page HTMX fragment handlers (accounts, storage, search, security).
//! Split out of `lib.rs` to keep that file within the size limit.

pub(crate) use axum::{extract::State, response::{Html, Json}};
pub(crate) use diesel::prelude::*;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::sync::Arc;
pub(crate) use botcore::shared::state::AppState;

/// Social account rows list the integration catalog providers and file them
/// under those catalog ids, so "Connected" reflects the real
/// `integration_connections` state written by the integrations app and its
/// OAuth callback rather than a static placeholder.
const SOCIAL_PROVIDERS: &[(&str, &str, &str)] = &[
    ("📷", "Instagram", "instagram"),
    ("📘", "Facebook", "facebook_pages"),
    ("🐦", "Twitter/X", "x"),
    ("💼", "LinkedIn", "linkedin_ads"),
];

#[derive(Debug, Deserialize)]
pub struct SearchSettingsRequest {
enable_fuzzy_search: Option<bool>,
search_result_limit: Option<i32>,
enable_ai_suggestions: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct SearchSettingsResponse {
success: bool,
message: Option<String>,
error: Option<String>,
}

mod part1;
mod part2;

pub use part1::*;
pub use part2::*;
