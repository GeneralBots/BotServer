pub(crate) use axum::extract::{Path, Query, State};
pub(crate) use axum::http::{header, StatusCode};
pub(crate) use axum::response::{IntoResponse, Redirect, Response};
pub(crate) use axum::Json;
pub(crate) use base64::Engine as _;
pub(crate) use diesel::prelude::*;
pub(crate) use hmac::{Hmac, Mac};
pub(crate) use serde::Deserialize;
pub(crate) use serde_json::{json, Value};
pub(crate) use sha2::Sha256;
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;
pub(crate) use botsecurity_auth::auth_api::types::AuthenticatedUser;
pub(crate) use crate::error::IntegrationError;
pub(crate) use crate::repository;
pub(crate) use crate::scope::{resolve_scope, ConnectionScope};
pub(crate) use crate::state::IntegrationState;

const STATE_MAX_AGE_SECS: i64 = 600;

pub(crate) struct OAuthProviderConfig {
    pub(crate) authorize_url: &'static str,
    pub(crate) token_url: &'static str,
    pub(crate) scopes: &'static str,
    pub(crate) basic_client_auth: bool,
}

#[derive(Deserialize)]
pub struct StartQuery {
    return_to: Option<String>,
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

mod part1;
mod part2;
mod part3;

pub(crate) use part1::*;
pub use part2::*;
pub use part3::*;
