use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TrustServiceCategory {
    Security,
    Availability,
    ProcessingIntegrity,
    Confidentiality,
    Privacy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Soc2Control {
    pub id: String,
    pub category: TrustServiceCategory,
    pub name: String,
    pub description: String,
    pub criteria: Vec<String>,
    pub implementation_status: ImplementationStatus,
    pub evidence_types: Vec<EvidenceType>,
    pub test_frequency: TestFrequency,
    pub last_tested: Option<DateTime<Utc>>,
    pub next_test_due: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImplementationStatus {
    NotImplemented,
    PartiallyImplemented,
    FullyImplemented,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum EvidenceType {
    Configuration,
    Logs,
    Policy,
    Procedure,
    Screenshot,
    Report,
    Certificate,
    Interview,
    Observation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TestFrequency {
    Continuous,
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Annually,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlTestResult {
    pub control_id: String,
    pub tested_at: DateTime<Utc>,
    pub tested_by: String,
    pub result: TestResult,
    pub findings: Vec<Finding>,
    pub evidence_collected: Vec<Evidence>,
    pub remediation_required: bool,
    pub remediation_deadline: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TestResult {
    Pass,
    PassWithException,
    Fail,
    NotTested,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: Uuid,
    pub severity: FindingSeverity,
    pub title: String,
    pub description: String,
    pub impact: String,
    pub recommendation: String,
    pub status: FindingStatus,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FindingSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FindingStatus {
    Open,
    InProgress,
    Resolved,
    Accepted,
    Deferred,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: Uuid,
    pub evidence_type: EvidenceType,
    pub title: String,
    pub description: String,
    pub collected_at: DateTime<Utc>,
    pub collected_by: String,
    pub storage_path: String,
    pub hash: String,
    pub retention_until: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SopLog {
    pub id: Uuid,
    pub procedure_id: String,
    pub procedure_name: String,
    pub executed_by: String,
    pub executed_at: DateTime<Utc>,
    pub steps_completed: Vec<SopStep>,
    pub result: SopResult,
    pub notes: Option<String>,
    pub duration_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SopStep {
    pub step_number: u32,
    pub description: String,
    pub completed: bool,
    pub completed_at: Option<DateTime<Utc>>,
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SopResult {
    Success,
    PartialSuccess,
    Failed,
    Aborted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentRecord {
    pub id: Uuid,
    pub incident_type: IncidentType,
    pub severity: FindingSeverity,
    pub title: String,
    pub description: String,
    pub detected_at: DateTime<Utc>,
    pub detected_by: String,
    pub affected_systems: Vec<String>,
    pub affected_users_count: Option<u64>,
    pub status: IncidentStatus,
    pub timeline: Vec<IncidentTimelineEntry>,
    pub root_cause: Option<String>,
    pub remediation_actions: Vec<String>,
    pub lessons_learned: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum IncidentType {
    SecurityBreach,
    DataLeak,
    ServiceOutage,
    UnauthorizedAccess,
    MalwareDetection,
    PolicyViolation,
    ConfigurationError,
    Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum IncidentStatus {
    Detected,
    Investigating,
    Containing,
    Eradicating,
    Recovering,
    Resolved,
    PostIncidentReview,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentTimelineEntry {
    pub timestamp: DateTime<Utc>,
    pub action: String,
    pub performed_by: String,
    pub notes: Option<String>,
}

pub struct Soc2ComplianceService {
    pub(crate) controls: Arc<RwLock<HashMap<String, Soc2Control>>>,
    pub(crate) test_results: Arc<RwLock<Vec<ControlTestResult>>>,
    pub(crate) sop_logs: Arc<RwLock<Vec<SopLog>>>,
    pub(crate) incidents: Arc<RwLock<Vec<IncidentRecord>>>,
    pub(crate) evidence_store: Arc<RwLock<Vec<Evidence>>>,
}

impl Soc2ComplianceService {
    pub fn new() -> Self {
        let service = Self {
            controls: Arc::new(RwLock::new(HashMap::new())),
            test_results: Arc::new(RwLock::new(Vec::new())),
            sop_logs: Arc::new(RwLock::new(Vec::new())),
            incidents: Arc::new(RwLock::new(Vec::new())),
            evidence_store: Arc::new(RwLock::new(Vec::new())),
        };

        tokio::spawn({
            let controls = service.controls.clone();
            async move {
                let mut guard = controls.write().await;
                for control in Self::get_default_controls() {
                    guard.insert(control.id.clone(), control);
                }
            }
        });

        service
    }

    fn get_default_controls() -> Vec<Soc2Control> {
        super::soc2_controls::get_default_controls()
    }
}

impl Default for Soc2ComplianceService {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Soc2ComplianceReport {
    pub generated_at: DateTime<Utc>,
    pub report_period_start: DateTime<Utc>,
    pub report_period_end: DateTime<Utc>,
    pub total_controls: usize,
    pub implemented_controls: usize,
    pub partially_implemented_controls: usize,
    pub not_implemented_controls: usize,
    pub compliance_score: f64,
    pub tests_performed: usize,
    pub tests_passed: usize,
    pub tests_failed: usize,
    pub open_findings: usize,
    pub critical_findings: usize,
    pub high_findings: usize,
    pub open_incidents: usize,
    pub controls_by_category: HashMap<TrustServiceCategory, CategoryStats>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CategoryStats {
    pub total: usize,
    pub implemented: usize,
    pub partially_implemented: usize,
    pub not_implemented: usize,
}

pub fn create_sop_log(
    procedure_id: &str,
    procedure_name: &str,
    executed_by: &str,
    steps: Vec<SopStep>,
    result: SopResult,
    duration_seconds: u64,
) -> SopLog {
    SopLog {
        id: Uuid::new_v4(),
        procedure_id: procedure_id.to_string(),
        procedure_name: procedure_name.to_string(),
        executed_by: executed_by.to_string(),
        executed_at: Utc::now(),
        steps_completed: steps,
        result,
        notes: None,
        duration_seconds,
    }
}

pub fn create_incident(
    incident_type: IncidentType,
    severity: FindingSeverity,
    title: &str,
    description: &str,
    detected_by: &str,
    affected_systems: Vec<String>,
) -> IncidentRecord {
    IncidentRecord {
        id: Uuid::new_v4(),
        incident_type,
        severity,
        title: title.to_string(),
        description: description.to_string(),
        detected_at: Utc::now(),
        detected_by: detected_by.to_string(),
        affected_systems,
        affected_users_count: None,
        status: IncidentStatus::Detected,
        timeline: vec![IncidentTimelineEntry {
            timestamp: Utc::now(),
            action: "Incident detected".to_string(),
            performed_by: detected_by.to_string(),
            notes: None,
        }],
        root_cause: None,
        remediation_actions: vec![],
        lessons_learned: None,
        resolved_at: None,
    }
}
