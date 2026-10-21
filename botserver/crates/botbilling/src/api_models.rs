use std::str::FromStr;

use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::{
    billing_invoice_items, billing_invoices, billing_payments, billing_quote_items,
    billing_quotes, billing_recurring, billing_tax_rates,
};

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = billing_invoices)]
pub struct BillingInvoice {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub invoice_number: String,
    pub customer_id: Option<Uuid>,
    pub customer_name: String,
    pub customer_email: Option<String>,
    pub customer_address: Option<String>,
    pub status: String,
    pub issue_date: NaiveDate,
    pub due_date: NaiveDate,
    pub subtotal: BigDecimal,
    pub tax_rate: BigDecimal,
    pub tax_amount: BigDecimal,
    pub discount_percent: BigDecimal,
    pub discount_amount: BigDecimal,
    pub total: BigDecimal,
    pub amount_paid: BigDecimal,
    pub amount_due: BigDecimal,
    pub currency: String,
    pub notes: Option<String>,
    pub terms: Option<String>,
    pub footer: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,
    pub sent_at: Option<DateTime<Utc>>,
    pub voided_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub branch_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = billing_invoice_items)]
pub struct BillingInvoiceItem {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub product_id: Option<Uuid>,
    pub description: String,
    pub quantity: BigDecimal,
    pub unit_price: BigDecimal,
    pub discount_percent: BigDecimal,
    pub tax_rate: BigDecimal,
    pub amount: BigDecimal,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = billing_payments)]
pub struct BillingPayment {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub invoice_id: Option<Uuid>,
    pub payment_number: String,
    pub amount: BigDecimal,
    pub currency: String,
    pub payment_method: String,
    pub payment_reference: Option<String>,
    pub status: String,
    pub payer_name: Option<String>,
    pub payer_email: Option<String>,
    pub notes: Option<String>,
    pub paid_at: DateTime<Utc>,
    pub refunded_at: Option<DateTime<Utc>>,
    pub refund_amount: Option<BigDecimal>,
    pub created_at: DateTime<Utc>,
    pub branch_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = billing_quotes)]
pub struct BillingQuote {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub quote_number: String,
    pub customer_id: Option<Uuid>,
    pub customer_name: String,
    pub customer_email: Option<String>,
    pub customer_address: Option<String>,
    pub status: String,
    pub issue_date: NaiveDate,
    pub valid_until: NaiveDate,
    pub subtotal: BigDecimal,
    pub tax_rate: BigDecimal,
    pub tax_amount: BigDecimal,
    pub discount_percent: BigDecimal,
    pub discount_amount: BigDecimal,
    pub total: BigDecimal,
    pub currency: String,
    pub notes: Option<String>,
    pub terms: Option<String>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub rejected_at: Option<DateTime<Utc>>,
    pub converted_invoice_id: Option<Uuid>,
    pub sent_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub branch_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = billing_quote_items)]
pub struct BillingQuoteItem {
    pub id: Uuid,
    pub quote_id: Uuid,
    pub product_id: Option<Uuid>,
    pub description: String,
    pub quantity: BigDecimal,
    pub unit_price: BigDecimal,
    pub discount_percent: BigDecimal,
    pub tax_rate: BigDecimal,
    pub amount: BigDecimal,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = billing_recurring)]
pub struct BillingRecurring {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub customer_id: Option<Uuid>,
    pub customer_name: String,
    pub customer_email: Option<String>,
    pub status: String,
    pub frequency: String,
    pub interval_count: i32,
    pub amount: BigDecimal,
    pub currency: String,
    pub description: Option<String>,
    pub next_invoice_date: NaiveDate,
    pub last_invoice_date: Option<NaiveDate>,
    pub last_invoice_id: Option<Uuid>,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub invoices_generated: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub branch_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = billing_tax_rates)]
pub struct BillingTaxRate {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub name: String,
    pub rate: BigDecimal,
    pub description: Option<String>,
    pub region: Option<String>,
    pub is_default: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub branch_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CreateInvoiceRequest {
    pub customer_name: Option<String>,
    pub customer_email: Option<String>,
    pub customer_address: Option<String>,
    pub customer_id: Option<Uuid>,
    pub issue_date: Option<String>,
    pub due_date: Option<String>,
    pub tax_rate: Option<f64>,
    pub discount_percent: Option<f64>,
    pub currency: Option<String>,
    pub notes: Option<String>,
    pub terms: Option<String>,
    pub items: Option<Vec<InvoiceItemRequest>>,
}

#[derive(Debug, Deserialize)]
pub struct InvoiceItemRequest {
    pub description: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub discount_percent: Option<f64>,
    pub tax_rate: Option<f64>,
    pub product_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateInvoiceRequest {
    pub customer_name: Option<String>,
    pub customer_email: Option<String>,
    pub customer_address: Option<String>,
    pub due_date: Option<String>,
    pub tax_rate: Option<f64>,
    pub discount_percent: Option<f64>,
    pub notes: Option<String>,
    pub terms: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RecordPaymentRequest {
    pub invoice_id: Option<Uuid>,
    pub amount: f64,
    pub payment_method: Option<String>,
    pub payment_reference: Option<String>,
    pub payer_name: Option<String>,
    pub payer_email: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateQuoteRequest {
    pub customer_name: Option<String>,
    pub customer_email: Option<String>,
    pub customer_address: Option<String>,
    pub customer_id: Option<Uuid>,
    pub issue_date: Option<String>,
    pub valid_until: Option<String>,
    pub tax_rate: Option<f64>,
    pub discount_percent: Option<f64>,
    pub currency: Option<String>,
    pub notes: Option<String>,
    pub terms: Option<String>,
    pub items: Option<Vec<InvoiceItemRequest>>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub search: Option<String>,
    pub status: Option<String>,
    pub customer_id: Option<Uuid>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct BillingStats {
    pub total_revenue: f64,
    pub revenue_this_month: f64,
    pub pending_amount: f64,
    pub overdue_amount: f64,
    pub paid_this_month: f64,
    pub invoice_count: i64,
    pub payment_count: i64,
    pub overdue_count: i64,
}

#[derive(Debug, Serialize)]
pub struct InvoiceWithItems {
    pub invoice: BillingInvoice,
    pub items: Vec<BillingInvoiceItem>,
}

#[derive(Debug, Serialize)]
pub struct QuoteWithItems {
    pub quote: BillingQuote,
    pub items: Vec<BillingQuoteItem>,
}

pub fn generate_invoice_number(conn: &mut diesel::PgConnection, branch_id: Uuid) -> String {
    let count: i64 = billing_invoices::table
        .filter(billing_invoices::branch_id.eq(branch_id))
        .count()
        .get_result(conn)
        .unwrap_or(0);
    format!("INV-{:06}", count + 1)
}

pub fn generate_payment_number(conn: &mut diesel::PgConnection, branch_id: Uuid) -> String {
    let count: i64 = billing_payments::table
        .filter(billing_payments::branch_id.eq(branch_id))
        .count()
        .get_result(conn)
        .unwrap_or(0);
    format!("PAY-{:06}", count + 1)
}

pub fn generate_quote_number(conn: &mut diesel::PgConnection, branch_id: Uuid) -> String {
    let count: i64 = billing_quotes::table
        .filter(billing_quotes::branch_id.eq(branch_id))
        .count()
        .get_result(conn)
        .unwrap_or(0);
    format!("QTE-{:06}", count + 1)
}

pub fn bd(val: f64) -> BigDecimal {
    BigDecimal::from_str(&val.to_string()).unwrap_or_else(|_| BigDecimal::from(0))
}

pub fn bd_to_f64(val: &BigDecimal) -> f64 {
    val.to_string().parse::<f64>().unwrap_or(0.0)
}
