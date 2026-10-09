pub(crate) use chrono::{DateTime, Duration, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use uuid::Uuid;
pub(crate) use crate::{BillingError, PlanConfig, PlanPrice, Subscription, SubscriptionStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionLifecycleEvent {
    pub id: Uuid,
    pub subscription_id: Uuid,
    pub organization_id: Uuid,
    pub event_type: LifecycleEventType,
    pub from_plan: Option<String>,
    pub to_plan: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
    pub processed: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleEventType {
    Created,
    Activated,
    Upgraded,
    Downgraded,
    Renewed,
    Paused,
    Resumed,
    CancellationRequested,
    Cancelled,
    Expired,
    PaymentFailed,
    PaymentRecovered,
    TrialStarted,
    TrialEnded,
    GracePeriodStarted,
    GracePeriodEnded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSubscriptionRequest {
    pub organization_id: Uuid,
    pub plan_id: String,
    pub payment_method_id: Option<String>,
    pub trial_days: Option<u32>,
    pub coupon_code: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeDowngradeRequest {
    pub subscription_id: Uuid,
    pub new_plan_id: String,
    pub prorate: bool,
    pub immediate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelSubscriptionRequest {
    pub subscription_id: Uuid,
    pub reason: Option<String>,
    pub feedback: Option<String>,
    pub cancel_immediately: bool,
    pub offer_retention: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionOffer {
    pub id: Uuid,
    pub subscription_id: Uuid,
    pub offer_type: RetentionOfferType,
    pub discount_percent: Option<u32>,
    pub free_months: Option<u32>,
    pub expires_at: DateTime<Utc>,
    pub accepted: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RetentionOfferType {
    Discount,
    FreeMonth,
    PlanDowngrade,
    FeatureUnlock,
    PersonalSupport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionChange {
    pub subscription_id: Uuid,
    pub change_type: ChangeType,
    pub effective_date: DateTime<Utc>,
    pub from_plan: String,
    pub to_plan: String,
    pub proration_amount: Option<i64>,
    pub status: ChangeStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    Upgrade,
    Downgrade,
    PlanChange,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeStatus {
    Pending,
    Scheduled,
    Applied,
    Cancelled,
    Failed,
}

pub struct SubscriptionLifecycleService {
    subscriptions: Arc<RwLock<HashMap<Uuid, Subscription>>>,
    events: Arc<RwLock<Vec<SubscriptionLifecycleEvent>>>,
    pending_changes: Arc<RwLock<HashMap<Uuid, SubscriptionChange>>>,
    retention_offers: Arc<RwLock<HashMap<Uuid, RetentionOffer>>>,
    plans: Arc<RwLock<HashMap<String, PlanConfig>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CancellationResult {
    CancelledImmediately,
    ScheduledForEndOfPeriod { end_date: DateTime<Utc> },
    RetentionOffered(RetentionOffer),
}

#[derive(Debug, Clone)]
pub enum LifecycleError {
    SubscriptionNotFound,
    PlanNotFound(String),
    InvalidOperation(String),
    OfferNotFound,
    OfferExpired,
    OfferAlreadyAccepted,
    BillingError(String),
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

pub use part5::*;
