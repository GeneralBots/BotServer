pub(crate) use chrono::{DateTime, Datelike, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InvoiceStatus {
    Draft,
    Open,
    Paid,
    Void,
    Uncollectible,
    PastDue,
    Refunded,
    PartiallyRefunded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InvoiceType {
    Subscription,
    OneTime,
    Usage,
    CreditNote,
    Adjustment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PaymentMethod {
    Card,
    BankTransfer,
    Check,
    Wire,
    Ach,
    Sepa,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceLineItem {
    pub id: Uuid,
    pub description: String,
    pub quantity: f64,
    pub unit_price: i64,
    pub amount: i64,
    pub currency: String,
    pub period_start: Option<DateTime<Utc>>,
    pub period_end: Option<DateTime<Utc>>,
    pub proration: bool,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceDiscount {
    pub id: Uuid,
    pub coupon_id: Option<String>,
    pub description: String,
    pub amount: i64,
    pub percent_off: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundResult {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub amount: i64,
    pub currency: String,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceTax {
    pub id: Uuid,
    pub description: String,
    pub rate: f64,
    pub amount: i64,
    pub jurisdiction: Option<String>,
    pub tax_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerAddress {
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceCustomer {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub tax_id: Option<String>,
    pub address: Option<CustomerAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentAttempt {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub amount: i64,
    pub status: String,
    pub failure_reason: Option<String>,
    pub payment_method: PaymentMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: Uuid,
    pub number: String,
    pub organization_id: Uuid,
    pub customer: InvoiceCustomer,
    pub status: InvoiceStatus,
    pub invoice_type: InvoiceType,
    pub currency: String,
    pub line_items: Vec<InvoiceLineItem>,
    pub discounts: Vec<InvoiceDiscount>,
    pub taxes: Vec<InvoiceTax>,
    pub subtotal: i64,
    pub total_discount: i64,
    pub total_tax: i64,
    pub total: i64,
    pub amount_paid: i64,
    pub amount_due: i64,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
    pub paid_at: Option<DateTime<Utc>>,
    pub voided_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub payment_attempts: Vec<PaymentAttempt>,
    pub stripe_invoice_id: Option<String>,
    pub hosted_invoice_url: Option<String>,
    pub pdf_url: Option<String>,
    pub notes: Option<String>,
    pub footer: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInvoiceRequest {
    pub organization_id: Uuid,
    pub customer: InvoiceCustomer,
    pub invoice_type: InvoiceType,
    pub currency: String,
    pub line_items: Vec<CreateLineItemRequest>,
    pub discounts: Option<Vec<CreateDiscountRequest>>,
    pub tax_rate: Option<f64>,
    pub due_days: Option<i64>,
    pub notes: Option<String>,
    pub footer: Option<String>,
    pub auto_send: bool,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLineItemRequest {
    pub description: String,
    pub quantity: f64,
    pub unit_price: i64,
    pub period_start: Option<DateTime<Utc>>,
    pub period_end: Option<DateTime<Utc>>,
    pub proration: bool,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDiscountRequest {
    pub coupon_id: Option<String>,
    pub description: String,
    pub amount: Option<i64>,
    pub percent_off: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceSummary {
    pub id: Uuid,
    pub number: String,
    pub customer_name: String,
    pub status: InvoiceStatus,
    pub total: i64,
    pub currency: String,
    pub due_date: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceListQuery {
    pub organization_id: Option<Uuid>,
    pub customer_id: Option<Uuid>,
    pub status: Option<InvoiceStatus>,
    pub invoice_type: Option<InvoiceType>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceListResponse {
    pub invoices: Vec<InvoiceSummary>,
    pub total_count: u64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceStats {
    pub total_invoices: u64,
    pub total_revenue: i64,
    pub outstanding_amount: i64,
    pub overdue_amount: i64,
    pub paid_count: u64,
    pub open_count: u64,
    pub overdue_count: u64,
    pub average_payment_days: f64,
}

pub struct InvoiceService {
    stripe_secret_key: Option<String>,
    company_name: String,
    company_address: CustomerAddress,
    company_tax_id: Option<String>,
    default_currency: String,
    default_due_days: i64,
    invoice_prefix: String,
    next_invoice_number: std::sync::atomic::AtomicU64,
}

pub(crate) struct StripeInvoiceResult {
    id: String,
    hosted_url: Option<String>,
    pdf_url: Option<String>,
}

#[derive(Debug, Clone)]
pub enum InvoiceError {
    NotFound(String),
    InvalidAmount(String),
    InvalidStatus(String),
    AlreadyPaid,
    AlreadyVoided,
    StripeError(String),
    PdfGenerationError(String),
}

mod part1;
mod part2;
mod part3;
mod part4;
