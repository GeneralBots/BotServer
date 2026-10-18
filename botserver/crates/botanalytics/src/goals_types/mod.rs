pub(crate) use bigdecimal::{BigDecimal, ToPrimitive};
pub(crate) use chrono::{DateTime, NaiveDate, Utc};
pub(crate) use diesel::prelude::*;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use uuid::Uuid;
pub(crate) use crate::schema::{okr_checkins, okr_key_results, okr_objectives, okr_templates};

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = okr_objectives)]
pub struct ObjectiveRecord {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub owner_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub period: String,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub status: String,
    pub progress: BigDecimal,
    pub visibility: String,
    pub weight: BigDecimal,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub org_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = okr_key_results)]
pub struct KeyResultRecord {
    pub id: Uuid,
    pub org_id: Uuid,
    pub branch_id: Uuid,
    pub objective_id: Uuid,
    pub owner_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub metric_type: String,
    pub start_value: BigDecimal,
    pub target_value: BigDecimal,
    pub current_value: BigDecimal,
    pub unit: Option<String>,
    pub weight: BigDecimal,
    pub status: String,
    pub due_date: Option<NaiveDate>,
    pub scoring_type: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = okr_checkins)]
pub struct CheckInRecord {
    pub id: Uuid,
    pub org_id: Uuid,
    pub branch_id: Uuid,
    pub key_result_id: Uuid,
    pub user_id: Uuid,
    pub previous_value: Option<BigDecimal>,
    pub new_value: BigDecimal,
    pub note: Option<String>,
    pub confidence: Option<String>,
    pub blockers: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable)]
#[diesel(table_name = okr_templates)]
pub struct TemplateRecord {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub objective_template: serde_json::Value,
    pub key_result_templates: serde_json::Value,
    pub is_system: bool,
    pub usage_count: i32,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub org_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Objective {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub owner_id: Uuid,
    pub owner_name: Option<String>,
    pub parent_id: Option<Uuid>,
    pub title: String,
    pub description: String,
    pub period: String,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub status: ObjectiveStatus,
    pub progress: f32,
    pub visibility: Visibility,
    pub weight: f32,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveStatus {
    Draft,
    Active,
    OnTrack,
    AtRisk,
    Behind,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Private,
    Team,
    Organization,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyResult {
    pub id: Uuid,
    pub objective_id: Uuid,
    pub owner_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub metric_type: MetricType,
    pub start_value: f64,
    pub target_value: f64,
    pub current_value: f64,
    pub unit: Option<String>,
    pub weight: f32,
    pub due_date: Option<NaiveDate>,
    pub status: KRStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetricType {
    Percentage,
    Number,
    Currency,
    Boolean,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KRStatus {
    NotStarted,
    InProgress,
    AtRisk,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckIn {
    pub id: Uuid,
    pub key_result_id: Uuid,
    pub user_id: Uuid,
    pub previous_value: f64,
    pub new_value: f64,
    pub note: String,
    pub confidence: Confidence,
    pub blockers: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalTemplate {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub objective_template: ObjectiveTemplate,
    pub key_result_templates: Vec<KeyResultTemplate>,
    pub is_system: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveTemplate {
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyResultTemplate {
    pub title: String,
    pub metric_type: MetricType,
    pub suggested_target: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentNode {
    pub objective: Objective,
    pub key_results: Vec<KeyResult>,
    pub children: Vec<AlignmentNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalsDashboard {
    pub total_objectives: i64,
    pub completed_objectives: i64,
    pub at_risk_objectives: i64,
    pub total_key_results: i64,
    pub at_risk_key_results: i64,
    pub average_progress: f32,
    pub upcoming_check_ins: Vec<UpcomingCheckIn>,
    pub recent_activity: Vec<GoalActivity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpcomingCheckIn {
    pub key_result_id: Uuid,
    pub key_result_title: String,
    pub objective_title: String,
    pub due_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalActivity {
    pub id: Uuid,
    pub activity_type: GoalActivityType,
    pub user_id: Uuid,
    pub user_name: String,
    pub objective_id: Option<Uuid>,
    pub key_result_id: Option<Uuid>,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalActivityType {
    ObjectiveCreated,
    ObjectiveUpdated,
    ObjectiveCompleted,
    KeyResultCreated,
    KeyResultUpdated,
    CheckInRecorded,
    ProgressChanged,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ListObjectivesQuery {
    pub owner_id: Option<Uuid>,
    pub status: Option<String>,
    pub period: Option<String>,
    pub parent_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateObjectiveRequest {
    pub title: String,
    pub description: Option<String>,
    pub period: String,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub parent_id: Option<Uuid>,
    pub visibility: Option<Visibility>,
    pub tags: Option<Vec<String>>,
    pub owner_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateObjectiveRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<ObjectiveStatus>,
    pub visibility: Option<Visibility>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateKeyResultRequest {
    pub title: String,
    pub description: Option<String>,
    pub metric_type: MetricType,
    pub start_value: Option<f64>,
    pub target_value: f64,
    pub unit: Option<String>,
    pub weight: Option<f32>,
    pub due_date: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateKeyResultRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub target_value: Option<f64>,
    pub current_value: Option<f64>,
    pub weight: Option<f32>,
    pub due_date: Option<NaiveDate>,
    pub status: Option<KRStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCheckInRequest {
    pub new_value: f64,
    pub note: Option<String>,
    pub confidence: Option<Confidence>,
    pub blockers: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AISuggestRequest {
    pub context: String,
    pub role: Option<String>,
    pub department: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AISuggestion {
    pub objective: ObjectiveTemplate,
    pub key_results: Vec<KeyResultTemplate>,
    pub rationale: String,
}

#[derive(Debug, thiserror::Error)]
pub enum GoalsError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Database error: {0}")]
    Database(String),
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;

pub(crate) use part6::*;
