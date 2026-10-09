pub(crate) use axum::{ extract::{Path, Query, State}, http::StatusCode, response::Json, };
pub(crate) use chrono::{DateTime, Utc};
pub(crate) use diesel::prelude::*;
pub(crate) use log::{error, info};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::sync::Arc;
pub(crate) use botcore::shared::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: Option<String>,
    pub first_name: String,
    pub last_name: String,
    pub display_name: Option<String>,
    pub role: Option<String>,
    pub organization_id: Option<String>,
    pub roles: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub organization_id: Option<String>,
    pub roles: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UserQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub search: Option<String>,
    pub organization_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub display_name: Option<String>,
    pub state: String,
    pub organization_id: Option<String>,
    pub roles: Vec<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct UserListResponse {
    pub users: Vec<UserResponse>,
    pub total: usize,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AssignOrganizationRequest {
    pub organization_id: String,
    pub roles: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRolesRequest {
    pub roles: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct UserPermissionsResponse {
    pub user_id: String,
    pub permissions: Vec<String>,
    pub roles: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Enable2faRequest {
    pub method: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNotificationPreferencesRequest {
    pub email: Option<bool>,
    pub push: Option<bool>,
    pub sms: Option<bool>,
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
