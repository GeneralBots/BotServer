pub(crate) use axum::{ body::Body, http::{Method, Request, StatusCode}, middleware::Next, response::Response, };
pub(crate) use chrono::{DateTime, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SopCategory {
    Authentication,
    Authorization,
    DataAccess,
    DataModification,
    DataDeletion,
    ConfigurationChange,
    SystemAccess,
    ApiAccess,
    FileAccess,
    UserManagement,
    RoleManagement,
    BillingOperation,
    ComplianceAction,
    SecurityEvent,
    AuditAction,
    BackupOperation,
    IncidentResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SopSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SopOutcome {
    Success,
    Failure,
    Denied,
    Error,
    Timeout,
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SopLogEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub category: SopCategory,
    pub severity: SopSeverity,
    pub outcome: SopOutcome,
    pub operation: String,
    pub description: String,
    pub user_id: Option<Uuid>,
    pub user_email: Option<String>,
    pub organization_id: Option<Uuid>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub request_method: Option<String>,
    pub request_path: Option<String>,
    pub request_id: Option<String>,
    pub response_status: Option<u16>,
    pub duration_ms: Option<u64>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub compliance_controls: Vec<String>,
    pub evidence_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SopSearchQuery {
    pub category: Option<SopCategory>,
    pub severity: Option<SopSeverity>,
    pub outcome: Option<SopOutcome>,
    pub user_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub resource_type: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub operation_contains: Option<String>,
    pub compliance_control: Option<String>,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SopSearchResult {
    pub entries: Vec<SopLogEntry>,
    pub total_count: u64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SopStatistics {
    pub total_entries: u64,
    pub entries_by_category: HashMap<String, u64>,
    pub entries_by_severity: HashMap<String, u64>,
    pub entries_by_outcome: HashMap<String, u64>,
    pub unique_users: u64,
    pub unique_organizations: u64,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

pub struct SopLogger {
    entries: Arc<RwLock<Vec<SopLogEntry>>>,
    max_entries: usize,
    retention_days: i64,
    webhook_url: Option<String>,
    siem_endpoint: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sop_log_entry_creation() {
        let entry = SopLogEntry::new(
            SopCategory::Authentication,
            "user_login",
            "User logged in",
        )
        .with_severity(SopSeverity::Info)
        .with_outcome(SopOutcome::Success)
        .with_user(Uuid::new_v4(), Some("test@example.com".to_string()));

        assert_eq!(entry.category, SopCategory::Authentication);
        assert_eq!(entry.severity, SopSeverity::Info);
        assert_eq!(entry.outcome, SopOutcome::Success);
        assert!(entry.user_id.is_some());
        assert_eq!(entry.user_email, Some("test@example.com".to_string()));
    }

    #[tokio::test]
    async fn test_sanitize_path() {
        let result = sanitize_path("test/operation");
        assert_eq!(result, "test_operation");
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;

pub use part6::*;
