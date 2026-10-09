//! Audit event model plus the in-memory audit trail service.

pub(crate) use anyhow::{anyhow, Result};
pub(crate) use chrono::{DateTime, Duration, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::{HashMap, VecDeque};
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum AuditEventType {
    UserLogin,
    UserLogout,
    PasswordChange,
    PermissionGranted,
    PermissionRevoked,
    DataAccess,
    DataModification,
    DataDeletion,
    ConfigurationChange,
    SecurityAlert,
    SystemError,
    ComplianceViolation,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub enum AuditSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub severity: AuditSeverity,
    pub user_id: Option<Uuid>,
    pub session_id: Option<String>,
    pub ip_address: Option<String>,
    pub resource_id: Option<String>,
    pub action: String,
    pub outcome: AuditOutcome,
    pub details: HashMap<String, String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum AuditOutcome {
    Success,
    Failure,
    Partial,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditTrail {
    pub trail_id: Uuid,
    pub name: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub events: Vec<Uuid>,
    pub summary: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub name: String,
    pub retention_days: i64,
    pub event_types: Vec<AuditEventType>,
    pub severity_threshold: Option<AuditSeverity>,
    pub archive_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStatistics {
    pub total_events: usize,
    pub events_by_type: HashMap<AuditEventType, usize>,
    pub events_by_severity: HashMap<AuditSeverity, usize>,
    pub events_by_outcome: HashMap<AuditOutcome, usize>,
    pub unique_users: usize,
    pub time_range: (DateTime<Utc>, DateTime<Utc>),
}

#[derive(Clone)]
pub struct AuditService {
    events: Arc<RwLock<VecDeque<AuditEvent>>>,
    trails: Arc<RwLock<HashMap<Uuid, AuditTrail>>>,
    retention_policies: Arc<RwLock<Vec<RetentionPolicy>>>,
    max_events: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditFilter {
    pub event_types: Option<Vec<AuditEventType>>,
    pub severity: Option<AuditSeverity>,
    pub user_id: Option<Uuid>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub outcome: Option<AuditOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    Json,
    Csv,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub generated_at: DateTime<Utc>,
    pub total_events: usize,
    pub security_incidents: usize,
    pub compliance_violations: usize,
    pub failed_logins: usize,
    pub unique_users: usize,
    pub critical_events: usize,
    pub audit_coverage: f64,
}

mod service;
mod filter;
