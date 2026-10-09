//! Aggregate statistics types.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnStatistics {
    pub total_courses: i64,
    pub total_lessons: i64,
    pub total_users_learning: i64,
    pub courses_completed: i64,
    pub certificates_issued: i64,
    pub average_completion_rate: f32,
    pub mandatory_compliance_rate: f32,
    pub popular_categories: Vec<CategoryStats>,
    pub recent_completions: Vec<RecentCompletion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryStats {
    pub category: String,
    pub courses_count: i64,
    pub enrolled_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentCompletion {
    pub user_id: Uuid,
    pub user_name: String,
    pub course_title: String,
    pub completed_at: DateTime<Utc>,
    pub score: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLearnStats {
    pub courses_enrolled: i64,
    pub courses_completed: i64,
    pub courses_in_progress: i64,
    pub total_time_spent_hours: f32,
    pub certificates_earned: i64,
    pub average_score: f32,
    pub pending_mandatory: i64,
    pub overdue_assignments: i64,
}
