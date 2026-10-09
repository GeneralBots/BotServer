pub(crate) use crate::BillingAlertNotification;
pub(crate) use crate::{DbPool, UsageMetric};
pub(crate) use chrono::{DateTime, Utc};
pub(crate) use diesel::prelude::*;
pub(crate) use diesel::sql_types::{BigInt, Bool, Nullable, Text, Timestamptz, Uuid as DieselUuid};
pub(crate) use diesel::sql_query;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub warning: f64,
    pub critical: f64,
    pub exceeded: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Warning,
    Critical,
    Exceeded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageAlert {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub metric: UsageMetric,
    pub severity: AlertSeverity,
    pub current_usage: u64,
    pub limit: u64,
    pub percentage: f64,
    pub threshold: f64,
    pub message: String,
    pub created_at: DateTime<Utc>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<Uuid>,
    pub notification_sent: bool,
    pub notification_channels: Vec<NotificationChannel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationChannel {
    Email,
    Webhook,
    InApp,
    Sms,
    Slack,
    MsTeams,
    Push,
}

pub struct AlertManager {
    active_alerts: Arc<RwLock<HashMap<Uuid, Vec<UsageAlert>>>>,
    alert_history: Arc<RwLock<HashMap<Uuid, Vec<UsageAlert>>>>,
    notification_prefs: Arc<RwLock<HashMap<Uuid, NotificationPreferences>>>,
    thresholds: AlertThresholds,
    cooldown_seconds: u64,
    max_history_per_org: usize,
    notification_handlers: Arc<RwLock<Vec<Arc<dyn NotificationHandler>>>>,
    pool: Option<DbPool>,
}

    /// INSERT the alert into `billing_alert_history` — the durable audit trail.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub enabled: bool,
    pub channels: Vec<NotificationChannel>,
    pub email_recipients: Vec<String>,
    pub webhook_url: Option<String>,
    pub webhook_secret: Option<String>,
    pub slack_webhook_url: Option<String>,
    pub teams_webhook_url: Option<String>,
    pub sms_numbers: Vec<String>,
    pub min_severity: AlertSeverity,
    pub quiet_hours: Option<QuietHours>,
    pub metric_overrides: HashMap<UsageMetric, MetricNotificationOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuietHours {
    pub start_hour: u8,
    pub end_hour: u8,
    pub timezone: String,
    pub days: Vec<chrono::Weekday>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricNotificationOverride {
    pub enabled: bool,
    pub min_severity: Option<AlertSeverity>,
    pub channels: Option<Vec<NotificationChannel>>,
}

#[derive(Debug, Clone)]
pub struct UsageSnapshot {
    pub messages: (u64, u64),
    pub storage_bytes: (u64, u64),
    pub api_calls: (u64, u64),
    pub bots: (u64, u64),
    pub users: (u64, u64),
    pub kb_documents: (u64, u64),
    pub apps: (u64, u64),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AlertCounts {
    pub total: usize,
    pub warning: usize,
    pub critical: usize,
    pub exceeded: usize,
}

#[async_trait::async_trait]
pub trait NotificationHandler: Send + Sync {
    fn channel(&self) -> NotificationChannel;
    async fn send(&self, notification: &AlertNotification) -> Result<(), NotificationError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertNotification {
    pub alert_id: Uuid,
    pub organization_id: Uuid,
    pub severity: AlertSeverity,
    pub title: String,
    pub message: String,
    pub metric: String,
    pub current_usage: u64,
    pub limit: u64,
    pub percentage: f64,
    pub action_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub recipients: Vec<String>,
}

pub struct WebhookNotificationHandler {}

pub struct InAppNotificationHandler {
    broadcast: Option<tokio::sync::broadcast::Sender<BillingAlertNotification>>,
}

#[derive(Debug, Clone)]
pub enum AlertError {
    NotFound,
    AlreadyExists,
    InvalidThreshold,
    StorageError(String),
}

#[derive(Debug, Clone)]
pub enum NotificationError {
    NetworkError(String),
    ConfigurationError(String),
    RateLimited,
    InvalidRecipient(String),
    DeliveryFailed(String),
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;
mod part8;
mod part9;
mod part10;
mod part11;
mod part12;
mod part13;
mod part14;
mod part15;

pub use part15::*;
