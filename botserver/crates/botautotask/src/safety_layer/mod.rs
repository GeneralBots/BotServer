pub(crate) use crate::types::UserSession;
pub(crate) use chrono::{DateTime, Utc};
pub(crate) use diesel::prelude::*;
pub(crate) use log::{info, trace, warn};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintCheckResult {
    pub passed: bool,
    pub results: Vec<ConstraintResult>,
    pub risk_score: f64,
    pub blocking: Vec<String>,
    pub warnings: Vec<String>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintResult {
    pub constraint_id: String,
    pub constraint_type: ConstraintType,
    pub passed: bool,
    pub severity: ConstraintSeverity,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub remediation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConstraintType {
    Budget,
    Permission,
    Policy,
    Compliance,
    Technical,
    RateLimit,
    TimeWindow,
    DataAccess,
    Security,
    Resource,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Ord, PartialOrd)]
#[derive(Default)]
pub enum ConstraintSeverity {
    Info = 0,
    #[default]
    Warning = 1,
    Error = 2,
    Critical = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub id: String,
    pub name: String,
    pub constraint_type: ConstraintType,
    pub description: String,
    pub expression: Option<String>,
    pub threshold: Option<serde_json::Value>,
    pub severity: ConstraintSeverity,
    pub enabled: bool,
    pub applies_to: Vec<String>,
    pub bot_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub id: String,
    pub success: bool,
    pub step_outcomes: Vec<StepSimulationOutcome>,
    pub impact: ImpactAssessment,
    pub resource_usage: PredictedResourceUsage,
    pub side_effects: Vec<SideEffect>,
    pub recommendations: Vec<Recommendation>,
    pub confidence: f64,
    pub simulated_at: DateTime<Utc>,
    pub simulation_duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepSimulationOutcome {
    pub step_id: String,
    pub step_name: String,
    pub would_succeed: bool,
    pub success_probability: f64,
    pub predicted_outputs: serde_json::Value,
    pub failure_modes: Vec<FailureMode>,
    pub estimated_duration_seconds: i32,
    pub affected_dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureMode {
    pub failure_type: String,
    pub probability: f64,
    pub impact: String,
    pub mitigation: Option<String>,
    pub recoverable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAssessment {
    pub risk_score: f64,
    pub risk_level: RiskLevel,
    pub data_impact: DataImpact,
    pub cost_impact: CostImpact,
    pub time_impact: TimeImpact,
    pub security_impact: SecurityImpact,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Ord, PartialOrd)]
#[derive(Default)]
pub enum RiskLevel {
    None = 0,
    #[default]
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataImpact {
    pub records_created: i32,
    pub records_modified: i32,
    pub records_deleted: i32,
    pub tables_affected: Vec<String>,
    pub data_sources_affected: Vec<String>,
    pub reversible: bool,
    pub backup_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostImpact {
    pub api_costs: f64,
    pub compute_costs: f64,
    pub storage_costs: f64,
    pub total_estimated_cost: f64,
    pub currency: String,
    pub exceeds_budget: bool,
    pub budget_remaining: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct TimeImpact {
    pub estimated_duration_seconds: i32,
    pub blocking: bool,
    pub delayed_tasks: Vec<String>,
    pub affects_deadline: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityImpact {
    pub risk_level: RiskLevel,
    pub credentials_accessed: Vec<String>,
    pub external_systems: Vec<String>,
    pub data_exposure_risk: bool,
    pub requires_elevation: bool,
    pub concerns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictedResourceUsage {
    pub cpu_percent: f64,
    pub memory_mb: f64,
    pub network_kbps: f64,
    pub disk_io_kbps: f64,
    pub api_calls: i32,
    pub db_queries: i32,
    pub llm_tokens: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideEffect {
    pub effect_type: String,
    pub description: String,
    pub severity: ConstraintSeverity,
    pub affected_systems: Vec<String>,
    pub intentional: bool,
    pub mitigation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub recommendation_type: RecommendationType,
    pub priority: i32,
    pub description: String,
    pub action: Option<String>,
    pub basic_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationType {
    AddSafetyCheck,
    AddErrorHandling,
    RequestApproval,
    AddBackup,
    Optimize,
    SplitSteps,
    AddMonitoring,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub actor: AuditActor,
    pub action: String,
    pub target: AuditTarget,
    pub outcome: AuditOutcome,
    pub details: serde_json::Value,
    pub related_entities: Vec<RelatedEntity>,
    pub session_id: String,
    pub bot_id: String,
    pub task_id: Option<String>,
    pub step_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub risk_level: RiskLevel,
    pub auto_executed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditEventType {
    TaskCreated, TaskStarted, TaskCompleted, TaskFailed, TaskCancelled, TaskPaused, TaskResumed,
    StepStarted, StepCompleted, StepFailed, StepSkipped, StepRolledBack,
    ApprovalRequested, ApprovalGranted, ApprovalDenied, ApprovalExpired,
    DecisionRequested, DecisionMade, DecisionTimeout,
    SimulationStarted, SimulationCompleted,
    ConstraintChecked, ConstraintViolated, ConstraintOverridden,
    DataRead, DataCreated, DataModified, DataDeleted,
    ApiCalled, McpInvoked, WebhookTriggered,
    PermissionChecked, PermissionDenied, CredentialAccessed, ConfigChanged,
    ErrorOccurred, WarningRaised, Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditActor {
    pub actor_type: ActorType,
    pub id: String,
    pub name: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActorType { User, Bot, System, External, Anonymous }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditTarget {
    pub target_type: String,
    pub id: String,
    pub name: Option<String>,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditOutcome {
    pub success: bool,
    pub result_code: Option<String>,
    pub message: Option<String>,
    pub duration_ms: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedEntity {
    pub entity_type: String,
    pub entity_id: String,
    pub relationship: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    pub enabled: bool,
    pub check_constraints: bool,
    pub simulate_impact: bool,
    pub audit_enabled: bool,
    pub approval_threshold: RiskLevel,
    pub max_auto_execute_risk: RiskLevel,
    pub default_budget_limit: f64,
    pub rate_limit_per_minute: i32,
    pub circuit_breaker_threshold: i32,
    pub audit_retention_days: i32,
    pub require_simulation_for: Vec<String>,
}

pub struct SafetyLayer {
    pool: crate::types::DbPool,
    config: SafetyConfig,
    constraints: Vec<Constraint>,
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
