pub(crate) use chrono::{DateTime, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceFramework {
    Gdpr,
    Soc2,
    Iso27001,
    Hipaa,
    PciDss,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceStatus {
    Compliant,
    PartialCompliance,
    NonCompliant,
    InProgress,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    Access,
    Modification,
    Deletion,
    Security,
    Admin,
    Authentication,
    Authorization,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionResult {
    Success,
    Failure,
    Denied,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RiskCategory {
    Technical,
    Operational,
    Financial,
    Compliance,
    Reputational,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TreatmentStrategy {
    Mitigate,
    Accept,
    Transfer,
    Avoid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RiskStatus {
    Open,
    InProgress,
    Mitigated,
    Accepted,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrainingType {
    SecurityAwareness,
    DataProtection,
    IncidentResponse,
    ComplianceOverview,
    RoleSpecific,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewAction {
    Approved,
    Revoked,
    Modified,
    FlaggedForReview,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    Pending,
    InProgress,
    Completed,
    Approved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceCheckResult {
    pub id: Uuid,
    pub framework: ComplianceFramework,
    pub control_id: String,
    pub control_name: String,
    pub status: ComplianceStatus,
    pub score: f64,
    pub checked_at: DateTime<Utc>,
    pub checked_by: Option<Uuid>,
    pub issues: Vec<ComplianceIssueResult>,
    pub evidence: Vec<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceIssueResult {
    pub id: Uuid,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub remediation: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub assigned_to: Option<Uuid>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub user_id: Option<Uuid>,
    pub resource_type: String,
    pub resource_id: String,
    pub action: String,
    pub result: ActionResult,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub id: Uuid,
    pub title: String,
    pub assessor_id: Uuid,
    pub methodology: String,
    pub overall_risk_score: f64,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub next_review_date: Option<chrono::NaiveDate>,
    pub risks: Vec<Risk>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Risk {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub category: RiskCategory,
    pub likelihood_score: i32,
    pub impact_score: i32,
    pub risk_score: i32,
    pub risk_level: Severity,
    pub current_controls: Vec<String>,
    pub treatment_strategy: TreatmentStrategy,
    pub status: RiskStatus,
    pub owner_id: Option<Uuid>,
    pub due_date: Option<chrono::NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub training_type: TrainingType,
    pub training_name: String,
    pub provider: Option<String>,
    pub score: Option<i32>,
    pub passed: bool,
    pub completion_date: DateTime<Utc>,
    pub valid_until: Option<DateTime<Utc>>,
    pub certificate_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessReview {
    pub id: Uuid,
    pub user_id: Uuid,
    pub reviewer_id: Uuid,
    pub review_date: DateTime<Utc>,
    pub permissions_reviewed: Vec<PermissionReview>,
    pub anomalies: Vec<String>,
    pub recommendations: Vec<String>,
    pub status: ReviewStatus,
    pub approved_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionReview {
    pub resource_type: String,
    pub resource_id: String,
    pub permissions: Vec<String>,
    pub justification: String,
    pub action: ReviewAction,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComplianceReport {
    pub generated_at: DateTime<Utc>,
    pub overall_score: f64,
    pub total_controls_checked: usize,
    pub compliant_controls: usize,
    pub total_issues: usize,
    pub critical_issues: usize,
    pub high_issues: usize,
    pub medium_issues: usize,
    pub low_issues: usize,
    pub results: Vec<ComplianceCheckResult>,
}

#[derive(Debug, Deserialize)]
pub struct ListChecksQuery {
    pub check_type: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ListIssuesQuery {
    pub severity: Option<String>,
    pub status: Option<String>,
    pub assigned_to: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ListAuditLogsQuery {
    pub action: Option<String>,
    pub actor_id: Option<Uuid>,
    pub target_type: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RunCheckRequest {
    pub framework: ComplianceFramework,
    pub control_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateIssueRequest {
    pub check_id: Option<Uuid>,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub remediation: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub assigned_to: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateIssueRequest {
    pub severity: Option<Severity>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub remediation: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub assigned_to: Option<Uuid>,
    pub status: Option<String>,
    pub resolution_notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAuditLogRequest {
    pub event_type: AuditEventType,
    pub user_id: Option<Uuid>,
    pub actor_id: Option<Uuid>,
    pub resource_type: String,
    pub target_type: String,
    pub resource_id: String,
    pub target_id: Option<Uuid>,
    pub action: String,
    pub result: ActionResult,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTrainingRequest {
    pub user_id: Uuid,
    pub training_type: TrainingType,
    pub training_name: String,
    pub provider: Option<String>,
    pub score: Option<i32>,
    pub passed: bool,
    pub valid_until: Option<DateTime<Utc>>,
    pub certificate_url: Option<String>,
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
