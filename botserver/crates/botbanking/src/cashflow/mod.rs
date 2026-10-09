//! Cash-flow sheet import into the banking ledger (issue #724).
//!
//! `POST /api/banking/imports/cashflow` imports the rows of a month's
//! cash-flow spreadsheet into the existing `bank_transactions` table with
//! `reconciled = false`, so they flow into the standard reconciliation
//! pipeline later. Only valid rows are imported (unparseable dates/amounts
//! are reported, never silently dropped). The sheet is read by explicit
//! Drive key (generic object access — no folder heuristics) or passed
//! directly as rows.
//!
//! `GET /api/banking/diagnosis` returns the cash-flow
//! result for the whole branch (an import affects the global cash flow, so
//! the diagnosis is never per-import): a summary plus the transaction
//! detail, along with the tax rates the tax engine resolves. It is pure
//! data — the LLM turns it into the diagnosis text and can run tax
//! experiments with the user through the calculate endpoint.

pub(crate) use axum::extract::{Query, State};
pub(crate) use axum::http::StatusCode;
pub(crate) use axum::Json;
pub(crate) use botcore::shared::state::AppState;
pub(crate) use chrono::{NaiveDate, Utc};
pub(crate) use diesel::prelude::*;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;

const IMPORT_BANK_MARKER: &str = "cashflow-import";

#[derive(Debug, Deserialize, Serialize)]
pub struct CashflowRow {
    pub date: String,
    pub description: Option<String>,
    pub amount: String,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CashflowImportRequest {
    /// Drive key of the sheet (e.g. `financeiro/fluxo-caixa-2026-08.csv`).
    /// When present the file is read from the bot's bucket; otherwise `rows`
    /// must be supplied.
    pub file_key: Option<String>,
    /// Optional YYYY-MM period; when absent it is derived from the rows
    /// (or defaults to the current month).
    pub period: Option<String>,
    /// Rows to import (used when `file_key` is absent).
    pub rows: Option<Vec<CashflowRow>>,
    /// Optional bot id; when absent the default bot is used.
    pub bot_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CashflowImportResponse {
    pub imported: usize,
    pub skipped_invalid: usize,
    pub invalid: Vec<serde_json::Value>,
    pub period: String,
    pub revenue: f64,
    pub expenses: f64,
    pub net: f64,
}

#[derive(Debug, Deserialize)]
pub struct DiagnosisQuery {
    pub period: Option<String>,
    pub bot_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DiagnosisRow {
    pub date: String,
    pub description: String,
    pub amount: f64,
    pub category: String,
    pub reconciled: bool,
    pub bank: String,
}

#[derive(Debug, Serialize)]
pub struct DiagnosisSummary {
    pub entries: usize,
    pub revenue: f64,
    pub expenses: f64,
    pub net: f64,
    pub pending_reconciliation: usize,
}

#[derive(Debug, Serialize)]
pub struct DiagnosisResponse {
    pub diagnosed: bool,
    pub period: Option<String>,
    pub scope: serde_json::Value,
    pub summary: DiagnosisSummary,
    pub detail: Vec<DiagnosisRow>,
    pub tax_rates: serde_json::Value,
    pub tax_rates_source: String,
    pub reason: Option<String>,
}

#[derive(diesel::QueryableByName, Clone)]
pub struct BotScopeRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub branch_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub name: String,
}

mod part1;
mod part2;

pub use part1::*;
pub use part2::*;
