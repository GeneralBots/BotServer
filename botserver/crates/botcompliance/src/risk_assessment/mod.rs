//! Risk assessment model, service and risk matrix.

pub(crate) use anyhow::{anyhow, Result};
pub(crate) use chrono::{DateTime, Duration, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RiskCategory {
    Security,
    Compliance,
    Operational,
    Financial,
    Reputational,
    Technical,
    Legal,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Likelihood {
    Rare,
    Unlikely,
    Possible,
    Likely,
    AlmostCertain,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Impact {
    Negligible,
    Minor,
    Moderate,
    Major,
    Catastrophic,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RiskStatus {
    Identified,
    Assessed,
    Mitigating,
    Monitoring,
    Accepted,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub category: RiskCategory,
    pub likelihood: Likelihood,
    pub impact: Impact,
    pub risk_level: RiskLevel,
    pub status: RiskStatus,
    pub identified_date: DateTime<Utc>,
    pub assessed_date: Option<DateTime<Utc>>,
    pub owner: String,
    pub affected_assets: Vec<String>,
    pub vulnerabilities: Vec<Vulnerability>,
    pub threats: Vec<Threat>,
    pub controls: Vec<Control>,
    pub residual_risk: Option<RiskLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub severity: RiskLevel,
    pub cve_id: Option<String>,
    pub discovered_date: DateTime<Utc>,
    pub patched: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Threat {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub threat_actor: String,
    pub likelihood: Likelihood,
    pub tactics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Control {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub control_type: ControlType,
    pub effectiveness: Effectiveness,
    pub implementation_status: ImplementationStatus,
    pub cost: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlType {
    Preventive,
    Detective,
    Corrective,
    Compensating,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effectiveness {
    Ineffective,
    PartiallyEffective,
    Effective,
    HighlyEffective,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImplementationStatus {
    Planned,
    InProgress,
    Implemented,
    Verified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MitigationPlan {
    pub id: Uuid,
    pub risk_id: Uuid,
    pub strategy: MitigationStrategy,
    pub actions: Vec<MitigationAction>,
    pub timeline: Duration,
    pub budget: f64,
    pub responsible_party: String,
    pub approval_status: ApprovalStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MitigationStrategy {
    Avoid,
    Transfer,
    Mitigate,
    Accept,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MitigationAction {
    pub id: Uuid,
    pub description: String,
    pub due_date: DateTime<Utc>,
    pub assigned_to: String,
    pub completed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone)]
pub struct RiskAssessmentService {
    assessments: HashMap<Uuid, RiskAssessment>,
    mitigation_plans: HashMap<Uuid, MitigationPlan>,
    risk_matrix: RiskMatrix,
}

#[derive(Debug, Clone)]
pub struct RiskMatrix {
    matrix: HashMap<(Likelihood, Impact), RiskLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskDashboard {
    pub total_risks: usize,
    pub risks_by_level: HashMap<RiskLevel, usize>,
    pub risks_by_category: HashMap<RiskCategory, usize>,
    pub risks_by_status: HashMap<RiskStatus, usize>,
    pub mitigation_plans_pending: usize,
    pub last_updated: DateTime<Utc>,
}

impl Default for RiskAssessmentService {
    fn default() -> Self {
        Self::new()
    }
}

mod service;

mod matrix;
