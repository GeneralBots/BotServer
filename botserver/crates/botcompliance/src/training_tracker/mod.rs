//! Compliance training model plus the in-memory training tracker service.

pub(crate) use anyhow::{anyhow, Result};
pub(crate) use chrono::{DateTime, Duration, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainingType {
    SecurityAwareness,
    DataProtection,
    PhishingPrevention,
    IncidentResponse,
    ComplianceRegulation,
    PasswordManagement,
    AccessControl,
    EmergencyProcedures,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrainingStatus {
    NotStarted,
    InProgress,
    Completed,
    Expired,
    Failed,
    Exempted,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TrainingPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingCourse {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub training_type: TrainingType,
    pub duration_hours: f32,
    pub validity_days: i64,
    pub priority: TrainingPriority,
    pub required_for_roles: Vec<String>,
    pub prerequisites: Vec<Uuid>,
    pub content_url: Option<String>,
    pub passing_score: u32,
    pub max_attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingAssignment {
    pub id: Uuid,
    pub user_id: Uuid,
    pub course_id: Uuid,
    pub assigned_date: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
    pub status: TrainingStatus,
    pub attempts: Vec<TrainingAttempt>,
    pub completion_date: Option<DateTime<Utc>>,
    pub expiry_date: Option<DateTime<Utc>>,
    pub assigned_by: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingAttempt {
    pub id: Uuid,
    pub attempt_number: u32,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub score: Option<u32>,
    pub passed: bool,
    pub time_spent_minutes: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingCertificate {
    pub id: Uuid,
    pub user_id: Uuid,
    pub course_id: Uuid,
    pub issued_date: DateTime<Utc>,
    pub expiry_date: DateTime<Utc>,
    pub certificate_number: String,
    pub verification_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStatus {
    pub user_id: Uuid,
    pub compliant: bool,
    pub required_trainings: Vec<Uuid>,
    pub completed_trainings: Vec<Uuid>,
    pub overdue_trainings: Vec<Uuid>,
    pub upcoming_trainings: Vec<Uuid>,
    pub compliance_percentage: f64,
}

#[derive(Debug, Clone)]
pub struct TrainingTracker {
    courses: HashMap<Uuid, TrainingCourse>,
    assignments: HashMap<Uuid, TrainingAssignment>,
    certificates: HashMap<Uuid, TrainingCertificate>,
    user_roles: HashMap<Uuid, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingReport {
    pub generated_at: DateTime<Utc>,
    pub total_courses: usize,
    pub total_assignments: usize,
    pub total_certificates: usize,
    pub assignments_by_status: HashMap<TrainingStatus, usize>,
    pub overdue_count: usize,
    pub expiring_soon: usize,
    pub average_score: f64,
}

impl Default for TrainingTracker {
    fn default() -> Self {
        Self::new()
    }
}

mod service;
