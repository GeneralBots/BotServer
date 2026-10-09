//! LLM-facing api command catalog (unified UI/API surface for chat).
//!
//! On web the LLM drives the UI via `__ui_plan__`; on chat/WhatsApp the
//! backend REST surface is otherwise unreachable. This module exposes:
//!
//! - a compact set of **executable commands** (whitelisted, run in-process
//!   against backend state) injected into every channel's system prompt;
//! - the full **API endpoint catalog** (method + path + one-line summary)
//!   searched on demand through `api.find` — the LLM learns the whole
//!   surface without a giant system prompt;
//! - `apps.find` (suite app registry search) and `web.search` (DuckDuckGo),
//!   so the model can decide between UI automation, backend data and the web.
//!
//! The LLM invokes a command by emitting a single
//! `{"__api_call__": {"name": ..., "params": {...}, "compose": true}}`
//! JSON block as the first line of its reply. General-knowledge questions
//! must be answered directly — no api call.

pub(crate) use std::sync::Arc;
pub(crate) use base64::Engine;
pub(crate) use bigdecimal::BigDecimal;
pub(crate) use botcore::shared::state::AppState;
pub(crate) use diesel::RunQueryDsl;
pub(crate) use rust_decimal::Decimal;
pub(crate) use serde_json::{json, Value};
pub(crate) use uuid::Uuid;
pub(crate) use crate::apps::registry;

pub const API_CALL_TRIGGER: &str = "\"__api_call__\":";

/// The declarative command model lives in `crate::apps::commands`. We reuse
/// that single source of truth here so the chat/WhatsApp prompt, the palette
/// and the discovery commands all agree on the same per-app actions.
pub use crate::apps::commands::{command_by_name, AppCommand};

pub struct ApiEndpoint {
    pub method: &'static str,
    pub path: &'static str,
    pub summary: &'static str,
}

#[derive(diesel::QueryableByName)]
struct ProjectNameRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    project_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    project_name: String,
}

#[derive(diesel::QueryableByName)]
struct PersonRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    person_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    first_name: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    last_name: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    email: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Text)]
    status: String,
}

#[derive(diesel::QueryableByName)]
struct InvoiceRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    invoice_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    invoice_number: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    customer_name: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    status: String,
    #[diesel(sql_type = diesel::sql_types::Numeric)]
    total: BigDecimal,
}

#[derive(diesel::QueryableByName)]
struct ProductRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    product_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    sku: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Text)]
    name: String,
    #[diesel(sql_type = diesel::sql_types::Numeric)]
    price: BigDecimal,
    #[diesel(sql_type = diesel::sql_types::Text)]
    product_type: String,
    #[diesel(sql_type = diesel::sql_types::Int4)]
    stock_quantity: i32,
}

#[derive(diesel::QueryableByName)]
struct TicketRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    ticket_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    ticket_number: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    subject: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    status: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    priority: String,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;

pub use part1::*;
pub use part2::*;
use part3::*;
pub use part4::*;
pub(crate) use part5::*;
use part6::*;
use part7::*;
