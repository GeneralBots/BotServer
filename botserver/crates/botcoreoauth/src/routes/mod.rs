pub(crate) use crate::{OAuthProvider, OAuthState, OAuthUserInfo};
pub(crate) use crate::providers::{get_enabled_providers, load_oauth_config};
pub(crate) use anyhow::anyhow;
pub(crate) use axum::{ extract::{Path, Query, State}, http::{header, StatusCode}, response::{Html, IntoResponse, Redirect, Response}, routing::get, Json, Router, };
pub(crate) use log::{debug, info, warn};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;

pub use botlib::db_pool::DbPool;

pub struct OAuthState_ {
    pub conn: DbPool,
    pub base_url: String,
}

#[derive(Debug, Deserialize)]
pub struct OAuthStartParams {
    pub redirect: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EnabledProvidersResponse {
    pub providers: Vec<ProviderInfo>,
}

#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub login_url: String,
}

mod part1;
mod part2;

pub use part1::*;
use part2::*;
