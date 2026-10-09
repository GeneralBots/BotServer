// Drive HTTP handlers — Issue #589 per-user file scoping

pub(crate) use botcore::shared::state::AppState;
pub(crate) use crate::drive_types::*;
pub(crate) use crate::user_scope;
pub(crate) use axum::{ extract::{Path, Query, State, Extension}, http::StatusCode, response::Json, };
pub(crate) use base64::Engine;
pub(crate) use botsecurity_auth::auth_api::types::AuthenticatedUser;
pub(crate) use diesel::prelude::*;
pub(crate) use diesel::sql_types::{BigInt, Bool, Nullable, Text, Timestamptz};
pub(crate) use log::{info, warn};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use std::sync::OnceLock;
pub(crate) use std::time::{Duration, Instant};

/// How long a computed quota snapshot is reused.
///
/// Recomputing walks every bucket in the instance and lists every object in
/// each one — 114s measured in prod across 32 buckets — and the Drive header
/// asks for it on every folder navigation. Serving a short-lived snapshot
/// turns that per-navigation cost into at most one sweep per window.
const QUOTA_TTL: Duration = Duration::from_secs(60);

/// How many buckets the quota sweep lists at once.
///
/// The sweep walked the buckets strictly one after another; 32 sequential
/// LISTs cost 114s in prod. A bounded pool keeps MinIO and the CPU budget busy
/// without opening one connection per bucket at once.
const QUOTA_SWEEP_CONCURRENCY: usize = 8;

static QUOTA_CACHE: OnceLock<std::sync::Mutex<Option<(Instant, QuotaResponse)>>> = OnceLock::new();

#[derive(diesel::QueryableByName)]
struct BotName {
    #[diesel(sql_type = diesel::sql_types::Text)]
    name: String,
}

// ====== Handlers ======

#[derive(serde::Deserialize)]
pub struct AIChatBody {
    pub message: String,
    pub file_path: Option<String>,
    pub bucket: Option<String>,
    pub scope: Option<FileScope>,
    pub user_id: Option<String>,
}

#[derive(serde::Serialize)]
pub struct AIChatResponse {
    pub reply: String,
}

#[derive(QueryableByName)]
struct PublicLinkRow {
    #[diesel(sql_type = Text)]
    bucket: String,
    #[diesel(sql_type = Text)]
    key: String,
    #[diesel(sql_type = Text)]
    path: String,
    #[diesel(sql_type = Text)]
    share_token: String,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
    #[diesel(sql_type = Bool)]
    revoked: bool,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;

use part1::*;
pub use part2::*;
pub use part3::*;
pub use part4::*;
pub use part5::*;
pub use part6::*;
pub use part7::*;
