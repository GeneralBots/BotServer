pub(crate) use axum::{ extract::{Path, Query, State}, http::{HeaderMap, StatusCode}, response::Json, };
pub(crate) use diesel::prelude::*;
pub(crate) use diesel::sql_query;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::sync::Arc;
pub(crate) use botcore::shared::sql_guard::sanitize_identifier;
pub(crate) use botcore::shared::state::AppState;
pub(crate) use crate::db;

const MAX_QUERY_ROWS: i64 = 10_000;
const DEFAULT_PAGE_SIZE: i64 = 100;

#[derive(Deserialize)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub limit: Option<i64>,
    pub sort: Option<String>,
    pub sort_order: Option<String>,
    pub filter_col: Option<String>,
    pub filter_op: Option<String>,
    pub filter_val: Option<String>,
}

#[derive(Deserialize)]
pub struct QueryRequest {
    pub query: String,
}

#[derive(Deserialize)]
pub struct CreateTableRequest {
    pub name: String,
    pub columns: Vec<ColumnDef>,
    #[serde(default)]
    pub table_name: Option<String>,
}

#[derive(Deserialize)]
pub struct ColumnDef {
    pub name: String,
    #[serde(default)]
    pub data_type: Option<String>,
    #[serde(default, alias = "type")]
    pub col_type: Option<String>,
    #[serde(default)]
    pub nullable: Option<bool>,
    #[serde(default)]
    pub not_null: Option<bool>,
    #[serde(default)]
    pub primary_key: Option<bool>,
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Deserialize)]
pub struct AlterTableRequest {
    pub add_columns: Option<Vec<ColumnDef>>,
    pub drop_columns: Option<Vec<String>>,
    pub rename_to: Option<String>,
}

#[derive(Deserialize)]
pub struct AddColumnRequest {
    pub name: String,
    #[serde(default)]
    pub data_type: Option<String>,
    #[serde(default, alias = "type")]
    pub col_type: Option<String>,
    #[serde(default)]
    pub nullable: Option<bool>,
    #[serde(default)]
    pub not_null: Option<bool>,
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Deserialize)]
pub struct InsertRowRequest {
    pub data: serde_json::Value,
}

#[derive(Deserialize)]
pub struct UpdateCellRequest {
    pub column: String,
    pub value: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct UpdateRowRequest {
    pub pk_value: String,
    pub data: serde_json::Value,
}

#[derive(Deserialize)]
pub struct BatchDeleteRequest {
    pub ids: Vec<String>,
}

#[derive(Serialize)]
pub struct SchemaResponse {
    pub tables: Vec<TableSchema>,
}

#[derive(Serialize)]
pub struct TableSchema {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
    pub column_count: usize,
    pub row_count: i64,
    pub table_size: String,
}

#[derive(Serialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub is_pk: bool,
    pub is_fk: bool,
}

#[derive(Serialize)]
pub struct TableDataResponse {
    pub columns: Vec<String>,
    pub rows: Vec<serde_json::Value>,
    pub total: i64,
    pub total_rows: i64,
    pub page: i64,
    pub page_size: i64,
    pub pk_column: Option<String>,
}

#[derive(Serialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<serde_json::Value>,
    pub row_count: i64,
    pub is_mutation: bool,
    pub duration_ms: u128,
}

#[derive(Serialize)]
pub struct ForeignKeyInfo {
    pub constraint_name: String,
    pub column_name: String,
    pub foreign_table: String,
    pub foreign_column: String,
}

#[derive(Serialize)]
pub struct ApiResult {
    pub success: bool,
    pub message: String,
}

#[derive(diesel::QueryableByName)]
struct ExistsRow {
    #[diesel(sql_type = diesel::sql_types::Bool)]
    exists: bool,
}

#[derive(QueryableByName, Debug)]
struct SchemaTable {
    #[diesel(sql_type = diesel::sql_types::Text)]
    table_name: String,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    row_count: i64,
    #[diesel(sql_type = diesel::sql_types::Text)]
    table_size: String,
}

#[derive(QueryableByName, Debug)]
struct SchemaColumn {
    #[diesel(sql_type = diesel::sql_types::Text)]
    table_name: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    column_name: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    data_type: String,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    is_nullable: bool,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    column_default: Option<String>,
}

#[derive(QueryableByName, Debug)]
struct PkInfo {
    #[diesel(sql_type = diesel::sql_types::Text)]
    table_name: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    column_name: String,
}

#[derive(QueryableByName, Debug)]
struct FkInfo {
    #[diesel(sql_type = diesel::sql_types::Text)]
    table_name: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    column_name: String,
}

#[derive(QueryableByName, Debug)]
struct CountResult {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

#[derive(QueryableByName, Debug)]
struct ColName {
    #[diesel(sql_type = diesel::sql_types::Text)]
    column_name: String,
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
pub use part3::*;
pub use part4::*;
pub use part5::*;
pub use part6::*;
pub use part7::*;
