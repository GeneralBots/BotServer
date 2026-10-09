pub(crate) use axum::{ extract::{Path, Query, State}, http::StatusCode, routing::{get, put}, Json, Router, };
pub(crate) use chrono::{Datelike, NaiveDate, Utc};
pub(crate) use diesel::prelude::*;
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;
pub(crate) use crate::api_models::{ bd, bd_to_f64, generate_invoice_number, generate_payment_number, generate_quote_number, BillingInvoice, BillingInvoiceItem, BillingPayment, BillingQuote, BillingQuoteItem, BillingRecurring, BillingStats, BillingTaxRate, CreateInvoiceRequest, CreateQuoteRequest, InvoiceWithItems, ListQuery, QuoteWithItems, RecordPaymentRequest, UpdateInvoiceRequest, };
pub(crate) use crate::schema::{ billing_invoice_items, billing_invoices, billing_payments, billing_quote_items, billing_quotes, billing_recurring, billing_tax_rates, };
pub(crate) use crate::{get_bot_context, DbPool, GetDefaultBotFn};

pub struct BillingApiState {
    pub pool: Arc<DbPool>,
    pub get_default_bot: Option<GetDefaultBotFn>,
}

mod part1;
mod part2;
mod part3;

pub use part1::*;
pub use part2::*;
pub use part3::*;
