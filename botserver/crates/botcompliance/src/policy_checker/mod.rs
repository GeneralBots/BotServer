//! Security policy model plus the in-memory policy checker service.

pub(crate) use anyhow::{anyhow, Result};
pub(crate) use chrono::{DateTime, Duration, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyType {
    AccessControl,
    DataRetention,
    PasswordStrength,
    SessionTimeout,
    EncryptionRequired,
    AuditLogging,
    BackupFrequency,
    NetworkSecurity,
    ComplianceStandard,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyStatus {
    Active,
    Draft,
    Deprecated,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PolicySeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub policy_type: PolicyType,
    pub status: PolicyStatus,
    pub severity: PolicySeverity,
    pub rules: Vec<PolicyRule>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub effective_date: DateTime<Utc>,
    pub expiry_date: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub id: Uuid,
    pub name: String,
    pub condition: String,
    pub action: PolicyAction,
    pub parameters: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyAction {
    Allow,
    Deny,
    Alert,
    Enforce,
    Log,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyViolation {
    pub id: Uuid,
    pub policy_id: Uuid,
    pub rule_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub user_id: Option<Uuid>,
    pub resource: String,
    pub action_attempted: String,
    pub violation_details: String,
    pub severity: PolicySeverity,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyCheckResult {
    pub policy_id: Uuid,
    pub passed: bool,
    pub violations: Vec<PolicyViolation>,
    pub warnings: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PolicyChecker {
    policies: HashMap<Uuid, SecurityPolicy>,
    violations: Vec<PolicyViolation>,
    check_history: Vec<PolicyCheckResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyContext {
    pub user_id: Option<Uuid>,
    pub resource: String,
    pub action: String,
    pub parameters: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyComplianceReport {
    pub generated_at: DateTime<Utc>,
    pub total_policies: usize,
    pub active_policies: usize,
    pub total_violations: usize,
    pub unresolved_violations: usize,
    pub critical_violations: usize,
    pub recent_checks: usize,
    pub compliance_rate: f64,
}

impl Default for PolicyChecker {
    fn default() -> Self {
        Self::new()
    }
}

mod service;
