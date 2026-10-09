//! Progress, assignment and certificate types.
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::learn_certificates;
use crate::schema::learn_course_assignments;
use crate::schema::learn_user_progress;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Insertable)]
#[diesel(table_name = learn_user_progress)]
pub struct UserProgress {
    pub id: Uuid,
    pub user_id: Uuid,
    pub course_id: Uuid,
    pub lesson_id: Option<Uuid>,
    pub status: String,
    pub quiz_score: Option<i32>,
    pub quiz_attempts: i32,
    pub time_spent_minutes: i32,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub last_accessed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProgressResponse {
    pub course_id: Uuid,
    pub course_title: String,
    pub status: ProgressStatus,
    pub completion_percentage: f32,
    pub lessons_completed: i32,
    pub lessons_total: i32,
    pub quiz_score: Option<i32>,
    pub quiz_passed: bool,
    pub time_spent_minutes: i32,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub last_accessed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProgressStatus {
    NotStarted,
    InProgress,
    Completed,
    Failed,
}

impl From<&str> for ProgressStatus {
    fn from(s: &str) -> Self {
        match s {
            "in_progress" => Self::InProgress,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            _ => Self::NotStarted,
        }
    }
}

impl std::fmt::Display for ProgressStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotStarted => write!(f, "not_started"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Insertable)]
#[diesel(table_name = learn_course_assignments)]
pub struct CourseAssignment {
    pub id: Uuid,
    pub course_id: Uuid,
    pub user_id: Uuid,
    pub assigned_by: Option<Uuid>,
    pub due_date: Option<DateTime<Utc>>,
    pub is_mandatory: bool,
    pub assigned_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub reminder_sent: bool,
    pub reminder_sent_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAssignmentRequest {
    pub course_id: Uuid,
    pub user_ids: Vec<Uuid>,
    pub due_date: Option<DateTime<Utc>>,
    pub is_mandatory: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentResponse {
    pub id: Uuid,
    pub course_id: Uuid,
    pub course_title: String,
    pub user_id: Uuid,
    pub assigned_by: Option<Uuid>,
    pub due_date: Option<DateTime<Utc>>,
    pub is_mandatory: bool,
    pub is_overdue: bool,
    pub days_until_due: Option<i64>,
    pub status: ProgressStatus,
    pub assigned_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Insertable)]
#[diesel(table_name = learn_certificates)]
pub struct Certificate {
    pub id: Uuid,
    pub user_id: Uuid,
    pub course_id: Uuid,
    pub issued_at: DateTime<Utc>,
    pub score: i32,
    pub certificate_url: Option<String>,
    pub verification_code: String,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_name: String,
    pub course_id: Uuid,
    pub course_title: String,
    pub issued_at: DateTime<Utc>,
    pub score: i32,
    pub verification_code: String,
    pub certificate_url: Option<String>,
    pub is_valid: bool,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateVerification {
    pub is_valid: bool,
    pub certificate: Option<CertificateResponse>,
    pub message: String,
}

