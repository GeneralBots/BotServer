//! Quiz and answer types.
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::schema::learn_quizzes;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable, Insertable)]
#[diesel(table_name = learn_quizzes)]
pub struct Quiz {
    pub id: Uuid,
    pub lesson_id: Option<Uuid>,
    pub course_id: Uuid,
    pub title: String,
    pub passing_score: i32,
    pub time_limit_minutes: Option<i32>,
    pub max_attempts: Option<i32>,
    pub questions: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizQuestion {
    pub id: Uuid,
    pub text: String,
    pub question_type: QuestionType,
    pub options: Vec<QuizOption>,
    pub correct_answers: Vec<usize>,
    pub explanation: Option<String>,
    pub points: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QuestionType {
    SingleChoice,
    MultipleChoice,
    TrueFalse,
    ShortAnswer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizOption {
    pub text: String,
    pub is_correct: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateQuizRequest {
    pub lesson_id: Option<Uuid>,
    pub title: String,
    pub passing_score: Option<i32>,
    pub time_limit_minutes: Option<i32>,
    pub max_attempts: Option<i32>,
    pub questions: Vec<QuizQuestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizResponse {
    pub id: Uuid,
    pub course_id: Uuid,
    pub lesson_id: Option<Uuid>,
    pub title: String,
    pub passing_score: i32,
    pub time_limit_minutes: Option<i32>,
    pub max_attempts: Option<i32>,
    pub questions_count: i32,
    pub total_points: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizSubmission {
    pub answers: HashMap<String, Vec<usize>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuizResult {
    pub quiz_id: Uuid,
    pub user_id: Uuid,
    pub score: i32,
    pub max_score: i32,
    pub percentage: f32,
    pub passed: bool,
    pub time_taken_minutes: i32,
    pub answers_breakdown: Vec<AnswerResult>,
    pub attempt_number: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerResult {
    pub question_id: Uuid,
    pub is_correct: bool,
    pub points_earned: i32,
    pub correct_answers: Vec<usize>,
    pub user_answers: Vec<usize>,
    pub explanation: Option<String>,
}

