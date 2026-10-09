pub(crate) use axum::{ extract::State, http::StatusCode, response::{Html, IntoResponse}, routing::{get, post}, Router, };
pub(crate) use diesel::RunQueryDsl;
pub(crate) use serde::Deserialize;
pub(crate) use std::sync::Arc;
pub(crate) use botcore::shared::state::AppState;
pub(crate) use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
pub(crate) use std::collections::HashSet;
pub(crate) use crate::settings_billing;
pub(crate) use crate::settings_credentials;
pub(crate) use crate::settings_profile;
pub(crate) use crate::settings_webhooks;

#[derive(Debug, diesel::QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct TextRow {
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    value: Option<String>,
}

#[derive(Deserialize)]
pub struct NotifPrefsForm {
    pub email_dm: Option<String>,
    pub email_mentions: Option<String>,
    pub email_digest: Option<String>,
    pub email_marketing: Option<String>,
    pub push_enabled: Option<String>,
    pub push_sound: Option<String>,
    pub desktop_notifications: Option<String>,
    pub badge_count: Option<String>,
}

#[derive(Deserialize)]
pub struct GroupCreateForm {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct UserCreateForm {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub role: Option<String>,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

pub use part1::*;
pub use part2::*;
pub use part3::*;
pub use part4::*;
pub use part5::*;
