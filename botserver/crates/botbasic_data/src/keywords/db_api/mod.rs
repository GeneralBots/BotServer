pub(crate) use super::table_access::{ check_field_write_access, check_table_access, filter_fields_by_role, AccessType, UserRoles, };
pub(crate) use botsecurity::sql_guard::{ build_safe_count_query_for_dynamic_table, build_safe_select_by_id_query_for_dynamic_table, build_safe_select_query_for_dynamic_table, is_table_allowed_with_conn };
pub(crate) use axum::{ extract::{Path, Query, State}, http::{HeaderMap, StatusCode}, response::IntoResponse, routing::{get, post}, Json, Router, };
pub(crate) use log::{info, warn};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub struct QueryParams {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub order_by: Option<String>,
    pub order_dir: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListResponse {
    pub data: Vec<Value>,
    pub total: i64,
    pub limit: i32,
    pub offset: i32,
}

#[derive(Debug, Serialize)]
pub struct RecordResponse {
    pub success: bool,
    pub data: Option<Value>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub deleted: i64,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub fields: Option<Vec<String>>,
    pub limit: Option<i32>,
}

#[derive(QueryableByName)]
struct JsonRow {
    #[diesel(sql_type = diesel::sql_types::Jsonb)]
    data: Value,
}

#[derive(QueryableByName)]
struct CountResult {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

use botcore::shared::state::AppState;
use botcore::shared::sanitize_identifier;
use botcore::urls::ApiUrls;
use botsecurity::error_sanitizer::log_and_sanitize;
use diesel::prelude::*;
use diesel::sql_query;
use std::sync::Arc;
use uuid::Uuid;

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

use part1::*;
pub use part2::*;
pub use part3::*;
pub use part4::*;
pub use part5::*;
