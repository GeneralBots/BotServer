pub(crate) use chrono::{DateTime, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountDeletionRequest {
    pub id: Uuid,
    pub user_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub email: String,
    pub reason: Option<String>,
    pub feedback: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub scheduled_for: DateTime<Utc>,
    pub status: DeletionStatus,
    pub confirmation_token: String,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub deletion_report: Option<DeletionReport>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeletionStatus {
    Pending,
    AwaitingConfirmation,
    Confirmed,
    InProgress,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionReport {
    pub user_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub data_deleted: Vec<DeletedDataCategory>,
    pub data_retained: Vec<RetainedDataCategory>,
    pub total_records_deleted: u64,
    pub confirmation_number: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletedDataCategory {
    pub category: DataCategory,
    pub records_deleted: u64,
    pub storage_freed_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetainedDataCategory {
    pub category: DataCategory,
    pub reason: RetentionReason,
    pub retention_period: Option<String>,
    pub will_be_deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataCategory {
    Profile,
    Sessions,
    Messages,
    Files,
    BotConfigurations,
    KnowledgeBase,
    Analytics,
    AuditLogs,
    Consents,
    Subscriptions,
    Invoices,
    ApiKeys,
    Integrations,
    Preferences,
    NotificationHistory,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RetentionReason {
    LegalObligation,
    TaxRecords,
    FraudPrevention,
    ActiveSubscription,
    PendingTransactions,
    DisputeResolution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDeletionRequest {
    pub reason: Option<String>,
    pub feedback: Option<String>,
    pub immediate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmDeletionRequest {
    pub confirmation_token: String,
    pub password: Option<String>,
}

pub struct AccountDeletionService {
    grace_period_days: u32,
    requests: std::sync::Arc<tokio::sync::RwLock<HashMap<Uuid, AccountDeletionRequest>>>,
}

#[derive(Debug, Clone)]
pub enum AccountDeletionError {
    RequestNotFound,
    RequestAlreadyExists,
    InvalidStatus,
    InvalidConfirmationToken,
    Unauthorized,
    CannotCancel,
    NotConfirmed,
    DeletionFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataExportRequest {
    pub id: Uuid,
    pub user_id: Uuid,
    pub requested_at: DateTime<Utc>,
    pub status: ExportStatus,
    pub format: ExportFormat,
    pub download_url: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportStatus {
    Pending,
    InProgress,
    Ready,
    Expired,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Json,
    Csv,
    Zip,
}

pub struct DataExportService {
    requests: std::sync::Arc<tokio::sync::RwLock<HashMap<Uuid, DataExportRequest>>>,
    download_expiry_hours: u32,
}

mod part1;
mod part2;
mod part3;
