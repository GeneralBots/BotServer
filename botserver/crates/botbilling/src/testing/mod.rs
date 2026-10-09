pub(crate) use chrono::{Duration, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockStripeCustomer {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub metadata: HashMap<String, String>,
    pub created: i64,
    pub default_source: Option<String>,
    pub invoice_settings: MockInvoiceSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockInvoiceSettings {
    pub default_payment_method: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockStripeSubscription {
    pub id: String,
    pub customer: String,
    pub status: SubscriptionStatus,
    pub current_period_start: i64,
    pub current_period_end: i64,
    pub items: MockSubscriptionItems,
    pub metadata: HashMap<String, String>,
    pub cancel_at_period_end: bool,
    pub canceled_at: Option<i64>,
    pub trial_start: Option<i64>,
    pub trial_end: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active,
    PastDue,
    Unpaid,
    Canceled,
    Incomplete,
    IncompleteExpired,
    Trialing,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockSubscriptionItems {
    pub data: Vec<MockSubscriptionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockSubscriptionItem {
    pub id: String,
    pub price: MockPrice,
    pub quantity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockPrice {
    pub id: String,
    pub product: String,
    pub unit_amount: i64,
    pub currency: String,
    pub recurring: Option<MockRecurring>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockRecurring {
    pub interval: String,
    pub interval_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockStripeInvoice {
    pub id: String,
    pub customer: String,
    pub subscription: Option<String>,
    pub status: InvoiceStatus,
    pub amount_due: i64,
    pub amount_paid: i64,
    pub amount_remaining: i64,
    pub currency: String,
    pub created: i64,
    pub due_date: Option<i64>,
    pub paid: bool,
    pub lines: MockInvoiceLines,
    pub hosted_invoice_url: Option<String>,
    pub invoice_pdf: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceStatus {
    Draft,
    Open,
    Paid,
    Uncollectible,
    Void,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockInvoiceLines {
    pub data: Vec<MockInvoiceLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockInvoiceLine {
    pub id: String,
    pub amount: i64,
    pub currency: String,
    pub description: Option<String>,
    pub quantity: u32,
    pub price: MockPrice,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockStripePaymentIntent {
    pub id: String,
    pub amount: i64,
    pub currency: String,
    pub status: PaymentIntentStatus,
    pub customer: Option<String>,
    pub payment_method: Option<String>,
    pub client_secret: String,
    pub created: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentIntentStatus {
    RequiresPaymentMethod,
    RequiresConfirmation,
    RequiresAction,
    Processing,
    RequiresCapture,
    Canceled,
    Succeeded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockStripePaymentMethod {
    pub id: String,
    pub customer: Option<String>,
    pub payment_type: String,
    pub card: Option<MockCard>,
    pub created: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockCard {
    pub brand: String,
    pub last4: String,
    pub exp_month: u32,
    pub exp_year: u32,
    pub funding: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockStripeEvent {
    pub id: String,
    pub event_type: String,
    pub created: i64,
    pub data: MockEventData,
    pub livemode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockEventData {
    pub object: serde_json::Value,
}

pub struct MockStripeClient {
    customers: Arc<RwLock<HashMap<String, MockStripeCustomer>>>,
    subscriptions: Arc<RwLock<HashMap<String, MockStripeSubscription>>>,
    invoices: Arc<RwLock<HashMap<String, MockStripeInvoice>>>,
    payment_intents: Arc<RwLock<HashMap<String, MockStripePaymentIntent>>>,
    payment_methods: Arc<RwLock<HashMap<String, MockStripePaymentMethod>>>,
    events: Arc<RwLock<Vec<MockStripeEvent>>>,
    prices: Arc<RwLock<HashMap<String, MockPrice>>>,
    should_fail: Arc<RwLock<bool>>,
    failure_code: Arc<RwLock<Option<String>>>,
}

#[derive(Debug, Clone)]
pub enum MockStripeError {
    NotFound(String),
    ApiError(String),
    InvalidRequest(String),
    AuthenticationError,
    RateLimitError,
    NetworkError(String),
}

#[derive(Debug, Clone)]
pub struct BillingTestScenario {
    pub name: String,
    pub description: String,
    pub steps: Vec<TestStep>,
}

#[derive(Debug, Clone)]
pub struct TestStep {
    pub action: TestAction,
    pub expected_result: ExpectedResult,
    pub delay_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum TestAction {
    CreateCustomer { email: String, name: Option<String> },
    CreateSubscription { plan: String, trial_days: Option<u32> },
    UpgradeSubscription { new_plan: String },
    DowngradeSubscription { new_plan: String },
    CancelSubscription { immediately: bool },
    SimulatePaymentFailure,
    SimulatePaymentSuccess,
    ProcessInvoice,
    ApplyDiscount { code: String },
    RecordUsage { metric: String, quantity: i64 },
    CheckQuota { metric: String },
}

#[derive(Debug, Clone)]
pub enum ExpectedResult {
    Success,
    SubscriptionStatus(SubscriptionStatus),
    InvoiceStatus(InvoiceStatus),
    QuotaExceeded,
    QuotaAvailable { remaining: i64 },
    Error { code: String },
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

use part3::*;
pub use part5::*;
