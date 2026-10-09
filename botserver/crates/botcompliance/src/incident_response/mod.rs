pub(crate) use chrono::{DateTime, Duration, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IncidentSeverity {
    Critical,
    High,
    Medium,
    Low,
    Informational,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IncidentStatus {
    New,
    Triaging,
    Investigating,
    Containing,
    Eradicating,
    Recovering,
    PostIncident,
    Resolved,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IncidentCategory {
    SecurityBreach,
    DataLeak,
    ServiceOutage,
    PerformanceDegradation,
    UnauthorizedAccess,
    Malware,
    PhishingAttack,
    DenialOfService,
    ConfigurationError,
    ComplianceViolation,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    pub id: Uuid,
    pub incident_number: String,
    pub title: String,
    pub description: String,
    pub severity: IncidentSeverity,
    pub status: IncidentStatus,
    pub category: IncidentCategory,
    pub affected_systems: Vec<String>,
    pub affected_users_count: Option<u64>,
    pub reported_by: String,
    pub assigned_to: Option<String>,
    pub team: Option<String>,
    pub source: IncidentSource,
    pub timeline: Vec<TimelineEntry>,
    pub actions_taken: Vec<ActionTaken>,
    pub evidence: Vec<Evidence>,
    pub related_incidents: Vec<Uuid>,
    pub tags: Vec<String>,
    pub sla_breach: bool,
    pub sla_breach_at: Option<DateTime<Utc>>,
    pub response_due_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub root_cause: Option<String>,
    pub lessons_learned: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IncidentSource {
    Manual,
    Automated,
    Alert,
    CustomerReport,
    SecurityScan,
    AuditFinding,
    ThirdParty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: TimelineEventType,
    pub description: String,
    pub user: Option<String>,
    pub automated: bool,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TimelineEventType {
    Created,
    StatusChanged,
    SeverityChanged,
    Assigned,
    CommentAdded,
    EvidenceAdded,
    ActionTaken,
    EscalationTriggered,
    NotificationSent,
    SlaBreached,
    Resolved,
    Closed,
    Reopened,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionTaken {
    pub id: Uuid,
    pub action_type: ActionType,
    pub description: String,
    pub performed_by: String,
    pub performed_at: DateTime<Utc>,
    pub automated: bool,
    pub success: bool,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActionType {
    IsolateSystem,
    BlockIpAddress,
    DisableAccount,
    RevokeCredentials,
    RestoreFromBackup,
    ApplyPatch,
    UpdateFirewallRules,
    EnableEnhancedLogging,
    NotifyStakeholders,
    EngageVendor,
    CollectEvidence,
    RunForensics,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: Uuid,
    pub evidence_type: EvidenceType,
    pub title: String,
    pub description: Option<String>,
    pub file_path: Option<String>,
    pub url: Option<String>,
    pub hash: Option<String>,
    pub collected_by: String,
    pub collected_at: DateTime<Utc>,
    pub chain_of_custody: Vec<CustodyEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EvidenceType {
    LogFile,
    Screenshot,
    NetworkCapture,
    MemoryDump,
    DiskImage,
    Configuration,
    Email,
    Document,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustodyEntry {
    pub timestamp: DateTime<Utc>,
    pub action: String,
    pub person: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationHook {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub trigger: HookTrigger,
    pub conditions: Vec<HookCondition>,
    pub actions: Vec<HookAction>,
    pub enabled: bool,
    pub priority: i32,
    pub cooldown_minutes: Option<i64>,
    pub last_triggered: Option<DateTime<Utc>>,
    pub trigger_count: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HookTrigger {
    IncidentCreated,
    StatusChanged,
    SeverityChanged,
    SlaBreachImminent,
    SlaBreach,
    Escalation,
    PatternDetected,
    ThresholdExceeded,
    Scheduled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookCondition {
    pub field: String,
    pub operator: ConditionOperator,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    Contains,
    GreaterThan,
    LessThan,
    In,
    NotIn,
    Matches,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookAction {
    pub action_type: HookActionType,
    pub parameters: HashMap<String, serde_json::Value>,
    pub retry_on_failure: bool,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HookActionType {
    SendNotification,
    CreateTicket,
    ExecuteRunbook,
    CallWebhook,
    UpdateIncident,
    AssignToTeam,
    EscalateToManager,
    BlockIpAddress,
    DisableUser,
    CollectLogs,
    TriggerBackup,
    SendSlackMessage,
    SendEmail,
    CreateJiraIssue,
    PageOnCall,
    RunScript,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationPolicy {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub levels: Vec<EscalationLevel>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationLevel {
    pub level: u32,
    pub delay_minutes: i64,
    pub notify_users: Vec<String>,
    pub notify_teams: Vec<String>,
    pub channels: Vec<NotificationChannel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NotificationChannel {
    Email,
    Slack,
    MsTeams,
    PagerDuty,
    Sms,
    Webhook,
    InApp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runbook {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub category: IncidentCategory,
    pub severity_levels: Vec<IncidentSeverity>,
    pub steps: Vec<RunbookStep>,
    pub automated_steps: Vec<AutomatedStep>,
    pub estimated_duration_minutes: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunbookStep {
    pub order: u32,
    pub title: String,
    pub description: String,
    pub responsible: String,
    pub verification: Option<String>,
    pub rollback: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomatedStep {
    pub order: u32,
    pub name: String,
    pub action: HookActionType,
    pub parameters: HashMap<String, serde_json::Value>,
    pub timeout_seconds: u64,
    pub continue_on_failure: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateIncidentRequest {
    pub title: String,
    pub description: String,
    pub severity: String,
    pub category: String,
    pub affected_systems: Option<Vec<String>>,
    pub reported_by: String,
    pub source: Option<String>,
    pub tags: Option<Vec<String>>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateIncidentRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub severity: Option<String>,
    pub status: Option<String>,
    pub assigned_to: Option<String>,
    pub team: Option<String>,
    pub root_cause: Option<String>,
    pub lessons_learned: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddTimelineEntryRequest {
    pub description: String,
    pub user: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddActionRequest {
    pub action_type: String,
    pub description: String,
    pub performed_by: String,
    pub success: bool,
    pub result: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListIncidentsQuery {
    pub status: Option<String>,
    pub severity: Option<String>,
    pub category: Option<String>,
    pub assigned_to: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct IncidentListResponse {
    pub incidents: Vec<IncidentSummary>,
    pub total: u64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

#[derive(Debug, Serialize)]
pub struct IncidentSummary {
    pub id: Uuid,
    pub incident_number: String,
    pub title: String,
    pub severity: String,
    pub status: String,
    pub category: String,
    pub assigned_to: Option<String>,
    pub sla_breach: bool,
    pub response_due_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct IncidentStats {
    pub total_incidents: u64,
    pub open_incidents: u64,
    pub resolved_incidents: u64,
    pub sla_breaches: u64,
    pub by_severity: HashMap<String, u64>,
    pub by_category: HashMap<String, u64>,
    pub by_status: HashMap<String, u64>,
    pub mean_time_to_resolve_hours: f64,
    pub mean_time_to_respond_minutes: f64,
}

pub struct IncidentResponseService {
    incidents: Arc<RwLock<HashMap<Uuid, Incident>>>,
    hooks: Arc<RwLock<Vec<AutomationHook>>>,
    escalation_policies: Arc<RwLock<HashMap<Uuid, EscalationPolicy>>>,
    runbooks: Arc<RwLock<HashMap<Uuid, Runbook>>>,
    incident_counter: std::sync::atomic::AtomicU64,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
