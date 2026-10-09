pub(crate) use chrono::{DateTime, Duration, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceFramework {
    Soc2TypeI,
    Soc2TypeII,
    Gdpr,
    Hipaa,
    Iso27001,
    PciDss,
    Ccpa,
    Nist,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TrustServiceCriteria {
    Security,
    Availability,
    ProcessingIntegrity,
    Confidentiality,
    Privacy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceType {
    Policy,
    Procedure,
    Screenshot,
    Log,
    Report,
    Configuration,
    Certificate,
    Attestation,
    AuditReport,
    TrainingRecord,
    AccessReview,
    RiskAssessment,
    IncidentReport,
    ChangeRecord,
    TestResult,
    Inventory,
    Contract,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    Draft,
    PendingReview,
    Approved,
    Rejected,
    Expired,
    Superseded,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CollectionMethod {
    Manual,
    Automated,
    Scheduled,
    Triggered,
    Imported,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub evidence_type: EvidenceType,
    pub status: EvidenceStatus,
    pub frameworks: Vec<ComplianceFramework>,
    pub control_ids: Vec<String>,
    pub tsc_categories: Vec<TrustServiceCriteria>,
    pub collection_method: CollectionMethod,
    pub collected_at: DateTime<Utc>,
    pub collected_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub reviewed_by: Option<Uuid>,
    pub valid_from: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub file_path: Option<String>,
    pub file_hash: Option<String>,
    pub file_size_bytes: Option<u64>,
    pub content_type: Option<String>,
    pub source_system: Option<String>,
    pub source_query: Option<String>,
    pub metadata: HashMap<String, String>,
    pub tags: Vec<String>,
    pub version: u32,
    pub previous_version_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlMapping {
    pub id: Uuid,
    pub framework: ComplianceFramework,
    pub control_id: String,
    pub control_name: String,
    pub description: String,
    pub tsc_category: Option<TrustServiceCriteria>,
    pub parent_control_id: Option<String>,
    pub required_evidence_types: Vec<EvidenceType>,
    pub collection_frequency_days: u32,
    pub automated_collection: bool,
    pub collection_sources: Vec<CollectionSource>,
    pub validation_rules: Vec<ValidationRule>,
    pub owner_id: Option<Uuid>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionSource {
    pub source_type: SourceType,
    pub source_name: String,
    pub connection_config: HashMap<String, String>,
    pub query_template: Option<String>,
    pub transform_script: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Database,
    Api,
    FileSystem,
    CloudProvider,
    IdentityProvider,
    SecurityTool,
    TicketSystem,
    LogAggregator,
    ConfigManagement,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub rule_id: String,
    pub rule_name: String,
    pub rule_type: ValidationRuleType,
    pub condition: String,
    pub error_message: String,
    pub severity: ValidationSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationRuleType {
    Required,
    Format,
    DateRange,
    FileSize,
    ContentMatch,
    Freshness,
    Completeness,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionTask {
    pub id: Uuid,
    pub control_id: String,
    pub evidence_type: EvidenceType,
    pub status: TaskStatus,
    pub scheduled_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub retry_count: u32,
    pub max_retries: u32,
    pub result_evidence_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRequest {
    pub id: Uuid,
    pub framework: ComplianceFramework,
    pub control_id: String,
    pub requested_by: Uuid,
    pub requested_at: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
    pub priority: RequestPriority,
    pub status: RequestStatus,
    pub assigned_to: Option<Uuid>,
    pub evidence_ids: Vec<Uuid>,
    pub notes: Option<String>,
    pub audit_period_start: DateTime<Utc>,
    pub audit_period_end: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RequestPriority {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RequestStatus {
    Open,
    InProgress,
    PendingReview,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidencePackage {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub framework: ComplianceFramework,
    pub audit_period_start: DateTime<Utc>,
    pub audit_period_end: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub status: PackageStatus,
    pub evidence_ids: Vec<Uuid>,
    pub control_coverage: HashMap<String, ControlCoverage>,
    pub export_format: Option<ExportFormat>,
    pub exported_at: Option<DateTime<Utc>>,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackageStatus {
    Draft,
    InProgress,
    Complete,
    Exported,
    Submitted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlCoverage {
    pub control_id: String,
    pub required: bool,
    pub evidence_count: u32,
    pub has_valid_evidence: bool,
    pub last_evidence_date: Option<DateTime<Utc>>,
    pub gaps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Pdf,
    Zip,
    Excel,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionSchedule {
    pub id: Uuid,
    pub control_id: String,
    pub evidence_type: EvidenceType,
    pub cron_expression: String,
    pub timezone: String,
    pub enabled: bool,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceGap {
    pub control_id: String,
    pub control_name: String,
    pub framework: ComplianceFramework,
    pub required_evidence_type: EvidenceType,
    pub gap_type: GapType,
    pub severity: GapSeverity,
    pub description: String,
    pub recommendation: String,
    pub due_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GapType {
    Missing,
    Expired,
    Incomplete,
    Invalid,
    Stale,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GapSeverity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionReport {
    pub id: Uuid,
    pub generated_at: DateTime<Utc>,
    pub report_period_start: DateTime<Utc>,
    pub report_period_end: DateTime<Utc>,
    pub framework: Option<ComplianceFramework>,
    pub summary: CollectionSummary,
    pub control_status: Vec<ControlStatus>,
    pub gaps: Vec<EvidenceGap>,
    pub collection_metrics: CollectionMetrics,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionSummary {
    pub total_controls: u32,
    pub controls_with_evidence: u32,
    pub controls_missing_evidence: u32,
    pub total_evidence_items: u32,
    pub evidence_collected_this_period: u32,
    pub evidence_expiring_soon: u32,
    pub coverage_percentage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlStatus {
    pub control_id: String,
    pub control_name: String,
    pub has_evidence: bool,
    pub evidence_count: u32,
    pub latest_evidence_date: Option<DateTime<Utc>>,
    pub evidence_valid: bool,
    pub gaps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionMetrics {
    pub automated_collections: u32,
    pub manual_collections: u32,
    pub failed_collections: u32,
    pub average_collection_time_seconds: f32,
    pub collection_success_rate: f32,
}

#[derive(Debug, Clone)]
pub enum CollectionError {
    NotFound(String),
    NotAutomated(String),
    ValidationFailed(String),
    StorageError(String),
    SourceError(String),
    InvalidInput(String),
}

pub struct EvidenceCollectionService {
    evidence: Arc<RwLock<HashMap<Uuid, EvidenceItem>>>,
    control_mappings: Arc<RwLock<HashMap<String, ControlMapping>>>,
    collection_tasks: Arc<RwLock<Vec<CollectionTask>>>,
    evidence_requests: Arc<RwLock<HashMap<Uuid, EvidenceRequest>>>,
    evidence_packages: Arc<RwLock<HashMap<Uuid, EvidencePackage>>>,
    collection_schedules: Arc<RwLock<Vec<CollectionSchedule>>>,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
