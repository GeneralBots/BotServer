pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct StripeClient {
    api_key: String,
    webhook_secret: Option<String>,
    client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeCustomer {
    pub id: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub metadata: HashMap<String, String>,
    pub created: i64,
    #[serde(default)]
    pub invoice_settings: Option<StripeCustomerInvoiceSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StripeCustomerInvoiceSettings {
    #[serde(default)]
    pub default_payment_method: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeSetupIntent {
    pub id: String,
    pub client_secret: Option<String>,
    pub status: String,
    #[serde(default)]
    pub customer: Option<String>,
    #[serde(default)]
    pub payment_method: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeSubscription {
    pub id: String,
    pub customer: String,
    pub status: StripeSubscriptionStatus,
    pub current_period_start: i64,
    pub current_period_end: i64,
    pub cancel_at_period_end: bool,
    pub canceled_at: Option<i64>,
    pub trial_start: Option<i64>,
    pub trial_end: Option<i64>,
    pub items: StripeSubscriptionItems,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeSubscriptionItems {
    pub data: Vec<StripeSubscriptionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeSubscriptionItem {
    pub id: String,
    pub price: StripePrice,
    pub quantity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripePrice {
    pub id: String,
    pub product: String,
    pub unit_amount: Option<i64>,
    pub currency: String,
    pub recurring: Option<StripePriceRecurring>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripePriceRecurring {
    pub interval: String,
    pub interval_count: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StripeSubscriptionStatus {
    Active,
    Canceled,
    Incomplete,
    IncompleteExpired,
    PastDue,
    Paused,
    Trialing,
    Unpaid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeInvoice {
    pub id: String,
    pub customer: String,
    pub subscription: Option<String>,
    pub status: StripeInvoiceStatus,
    pub amount_due: i64,
    pub amount_paid: i64,
    pub currency: String,
    pub created: i64,
    pub hosted_invoice_url: Option<String>,
    pub invoice_pdf: Option<String>,
    #[serde(default)]
    pub customer_email: Option<String>,
    #[serde(default)]
    pub customer_name: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StripeInvoiceStatus {
    Draft,
    Open,
    Paid,
    Uncollectible,
    Void,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripePaymentMethod {
    pub id: String,
    pub customer: Option<String>,
    #[serde(rename = "type")]
    pub payment_type: String,
    pub card: Option<StripeCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeCard {
    pub brand: String,
    pub last4: String,
    pub exp_month: u32,
    pub exp_year: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeCheckoutSession {
    pub id: String,
    pub url: Option<String>,
    pub customer: Option<String>,
    pub subscription: Option<String>,
    pub status: String,
    pub mode: String,
    #[serde(default)]
    pub setup_intent: Option<String>,
    #[serde(default)]
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeBillingPortalSession {
    pub id: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct CreateCustomerParams {
    pub email: String,
    pub name: Option<String>,
    pub organization_id: Uuid,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct CreateCheckoutSessionParams {
    pub customer_id: String,
    pub price_id: String,
    pub success_url: String,
    pub cancel_url: String,
    pub trial_days: Option<u32>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct CreatePortalSessionParams {
    pub customer_id: String,
    pub return_url: String,
}

#[derive(Debug, Clone)]
pub struct CreateSetupIntentParams {
    pub customer_id: String,
    /// `on_session` (default) or `off_session`. Off-session is required when
    /// the saved card is later charged without the customer present.
    pub usage: String,
}

#[derive(Debug, Clone)]
pub struct CreateSetupCheckoutSessionParams {
    pub customer_id: String,
    pub success_url: String,
    pub cancel_url: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeWebhookEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: StripeWebhookData,
    pub created: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeWebhookData {
    pub object: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum StripeError {
    ApiError(String),
    NetworkError(String),
    InvalidWebhook(String),
    ParseError(String),
    NotConfigured,
}

impl StripeClient {
    /// Stripe API base URL is a compile-time constant: it must never be
    /// configurable at runtime, which would open a server-side request
    /// forgery path (credentials would be sent to an attacker-chosen host).
    pub(crate) const API_BASE: &str = "https://api.stripe.com/v1";
}

#[derive(Debug)]
pub enum WebhookEventType {
    SubscriptionCreated(StripeSubscription),
    SubscriptionUpdated(StripeSubscription),
    SubscriptionCanceled(StripeSubscription),
    InvoicePaid(StripeInvoice),
    InvoicePaymentFailed(StripeInvoice),
    CheckoutCompleted(StripeCheckoutSession),
    PaymentMethodAttached(StripePaymentMethod),
    PaymentMethodUpdated(StripePaymentMethod),
    PaymentMethodDetached(StripePaymentMethod),
    Unknown(String),
}

pub struct StripeWebhookHandler {
    client: StripeClient,
}

#[derive(Debug, Clone)]
pub enum WebhookAction {
    ActivateSubscription {
        stripe_subscription_id: String,
        stripe_customer_id: String,
        status: StripeSubscriptionStatus,
        period_end: i64,
    },
    UpdateSubscription {
        stripe_subscription_id: String,
        status: StripeSubscriptionStatus,
        cancel_at_period_end: bool,
        period_end: i64,
    },
    CancelSubscription {
        stripe_subscription_id: String,
    },
    RecordPayment {
        stripe_invoice_id: String,
        amount: i64,
        currency: String,
    },
    PaymentFailed {
        stripe_invoice_id: String,
        stripe_customer_id: String,
    },
    CheckoutCompleted {
        stripe_customer_id: Option<String>,
        stripe_subscription_id: Option<String>,
    },
    PaymentMethodSync {
        pm: StripePaymentMethod,
        detach: bool,
    },
    None,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(payload: &str, event_type: &str) -> StripeWebhookEvent {
        serde_json::from_str(&format!(
            r#"{{"id":"evt_1","type":"{event_type}","created":123,"data":{{"object":{payload}}}}}"#
        ))
        .expect("valid webhook fixture")
    }

    #[test]
    fn test_parse_payment_method_attached() {
        let client = StripeClient::new("sk_test".to_string(), None);
        let evt = event(
            r#"{"id":"pm_123","customer":"cus_1","type":"card","card":{"brand":"visa","last4":"4242","exp_month":12,"exp_year":2030}}"#,
            "payment_method.attached",
        );
        match client.parse_webhook_event(&evt).expect("parses") {
            WebhookEventType::PaymentMethodAttached(pm) => {
                assert_eq!(pm.id, "pm_123");
                assert_eq!(pm.customer.as_deref(), Some("cus_1"));
                let card = pm.card.expect("card payload");
                assert_eq!(card.brand, "visa");
                assert_eq!(card.last4, "4242");
                assert_eq!(card.exp_year, 2030);
            }
            other => panic!("expected PaymentMethodAttached, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_customer_source_deleted_maps_to_detached() {
        let client = StripeClient::new("sk_test".to_string(), None);
        let evt = event(
            r#"{"id":"pm_9","customer":"cus_1","type":"card","card":{"brand":"mastercard","last4":"0005","exp_month":1,"exp_year":2028}}"#,
            "customer.source.deleted",
        );
        match client.parse_webhook_event(&evt).expect("parses") {
            WebhookEventType::PaymentMethodDetached(pm) => {
                assert_eq!(pm.id, "pm_9");
            }
            other => panic!("expected PaymentMethodDetached, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_setup_checkout_session() {
        let client = StripeClient::new("sk_test".to_string(), None);
        let evt = event(
            r#"{"id":"cs_1","url":"https://checkout.stripe.com/x","customer":"cus_1","subscription":null,"status":"complete","mode":"setup","setup_intent":"seti_1"}"#,
            "checkout.session.completed",
        );
        match client.parse_webhook_event(&evt).expect("parses") {
            WebhookEventType::CheckoutCompleted(session) => {
                assert_eq!(session.mode, "setup");
                assert_eq!(session.setup_intent.as_deref(), Some("seti_1"));
            }
            other => panic!("expected CheckoutCompleted, got {other:?}"),
        }
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
