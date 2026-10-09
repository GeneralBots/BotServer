//! Lesson types.
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::learn_lessons;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Insertable)]
#[diesel(table_name = learn_lessons)]
pub struct Lesson {
    pub id: Uuid,
    pub course_id: Uuid,
    pub title: String,
    pub content: Option<String>,
    pub content_type: String,
    pub lesson_order: i32,
    pub duration_minutes: i32,
    pub video_url: Option<String>,
    pub attachments: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLessonRequest {
    pub title: String,
    pub content: Option<String>,
    pub content_type: Option<String>,
    pub duration_minutes: Option<i32>,
    pub video_url: Option<String>,
    pub attachments: Option<Vec<AttachmentInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLessonRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub content_type: Option<String>,
    pub lesson_order: Option<i32>,
    pub duration_minutes: Option<i32>,
    pub video_url: Option<String>,
    pub attachments: Option<Vec<AttachmentInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentInfo {
    pub name: String,
    pub url: String,
    pub file_type: String,
    pub size_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LessonResponse {
    pub id: Uuid,
    pub course_id: Uuid,
    pub title: String,
    pub content: Option<String>,
    pub content_type: String,
    pub lesson_order: i32,
    pub duration_minutes: i32,
    pub video_url: Option<String>,
    pub attachments: Vec<AttachmentInfo>,
    pub is_completed: bool,
    pub created_at: DateTime<Utc>,
}

