//! Course, category and filter types.
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::learn_categories;
use crate::schema::learn_courses;

use super::lesson::LessonResponse;
use super::progress::UserProgressResponse;
use super::quiz::QuizResponse;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Insertable)]
#[diesel(table_name = learn_courses)]
pub struct Course {
    pub id: Uuid,
    pub organization_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub difficulty: String,
    pub duration_minutes: i32,
    pub thumbnail_url: Option<String>,
    pub is_mandatory: bool,
    pub due_days: Option<i32>,
    pub is_published: bool,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCourseRequest {
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub difficulty: Option<String>,
    pub duration_minutes: Option<i32>,
    pub thumbnail_url: Option<String>,
    pub is_mandatory: Option<bool>,
    pub due_days: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCourseRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub difficulty: Option<String>,
    pub duration_minutes: Option<i32>,
    pub thumbnail_url: Option<String>,
    pub is_mandatory: Option<bool>,
    pub due_days: Option<i32>,
    pub is_published: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseResponse {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub difficulty: String,
    pub duration_minutes: i32,
    pub thumbnail_url: Option<String>,
    pub is_mandatory: bool,
    pub due_days: Option<i32>,
    pub is_published: bool,
    pub lessons_count: i32,
    pub enrolled_count: i32,
    pub completion_rate: f32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseDetailResponse {
    pub course: CourseResponse,
    pub lessons: Vec<LessonResponse>,
    pub quiz: Option<QuizResponse>,
    pub user_progress: Option<UserProgressResponse>,
}


#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Insertable)]
#[diesel(table_name = learn_categories)]
pub struct Category {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub parent_id: Option<Uuid>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub courses_count: i32,
    pub children: Vec<CategoryResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseFilters {
    pub category: Option<String>,
    pub difficulty: Option<String>,
    pub is_mandatory: Option<bool>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressFilters {
    pub status: Option<String>,
    pub course_id: Option<Uuid>,
}

