//! Accounts, PIX transfers, statements, settings and dashboard stats for the
//! banking reconciliation suite app.

pub(crate) use axum::extract::{Json, Path};
pub(crate) use axum::http::{HeaderMap, StatusCode};
pub(crate) use chrono::Utc;
pub(crate) use rust_decimal::Decimal;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use uuid::Uuid;
pub(crate) use diesel::prelude::*;
pub(crate) use crate::db;
pub(crate) use botcore::shared::tenant::branch_from_claims;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankAccount {
    pub id: Uuid,
    pub bank: String,
    pub agency: String,
    pub account_number: String,
    pub account_type: String,
    pub balance: String,
    pub currency: String,
    pub last_sync: Option<chrono::DateTime<Utc>>,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct NewAccount {
    pub bank: String,
    #[serde(default)]
    pub agency: String,
    #[serde(default)]
    pub account_number: String,
    #[serde(default = "default_account_type")]
    pub account_type: String,
    #[serde(default)]
    pub balance: String,
    #[serde(default = "default_currency")]
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PixTransfer {
    pub id: Uuid,
    pub direction: String,
    pub key_type: String,
    pub key_value: String,
    pub counterparty: String,
    pub amount: String,
    pub description: String,
    pub status: String,
    pub end_to_end_id: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct NewPixTransfer {
    pub key_type: String,
    pub key_value: String,
    #[serde(default)]
    pub counterparty: String,
    pub amount: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub scheduled_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct NewPixReceive {
    pub key_type: String,
    pub key_value: String,
    #[serde(default)]
    pub counterparty: String,
    pub amount: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statement {
    pub id: Uuid,
    pub period: String,
    pub account_label: String,
    pub opening: String,
    pub closing: String,
    pub generated_at: chrono::DateTime<Utc>,
    pub format: String,
}

#[derive(Debug, Deserialize)]
pub struct NewStatement {
    pub account_id: Uuid,
    pub period: String,
    #[serde(default = "default_statement_format")]
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankingSettings {
    pub tolerance_cents: i32,
    pub date_window_days: i32,
    pub auto_approve_under: String,
    pub notify_on_unmatched: bool,
    pub webhook: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveSettings {
    #[serde(default)]
    pub tolerance_cents: Option<i32>,
    #[serde(default)]
    pub date_window_days: Option<i32>,
    #[serde(default)]
    pub auto_approve_under: Option<String>,
    #[serde(default)]
    pub notify_on_unmatched: Option<bool>,
    #[serde(default)]
    pub webhook: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BankingStats {
    pub total_in: f64,
    pub total_out: f64,
    pub net: f64,
    pub pending: i64,
    pub match_rate: f64,
}

#[derive(Debug, Deserialize)]
pub struct GenerateReport {
    pub kind: String,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;
mod part8;

use part1::*;
pub use part2::*;
pub use part3::*;
pub use part4::*;
pub use part5::*;
pub use part6::*;
pub use part7::*;
pub use part8::*;
