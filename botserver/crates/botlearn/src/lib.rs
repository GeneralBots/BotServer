pub mod basic_storage;
pub mod certification;
pub mod course;
pub mod gamification;
pub use gamification::GamificationService;
pub mod models;
pub mod routes_learn;
pub mod schema;
pub mod types;
pub mod ui;

pub use types::{
    AnswerResult, AttachmentInfo, Category, CategoryResponse, Certificate,
    CertificateResponse, CertificateVerification, Course, CourseAssignment,
    CourseDetailResponse, CourseFilters, CourseResponse, CreateAssignmentRequest,
    CreateCourseRequest, CreateLessonRequest, CreateQuizRequest, LearnStatistics,
    Lesson, LessonResponse, ProgressFilters, ProgressStatus, QuestionType, Quiz,
    QuizOption, QuizQuestion, QuizResponse, QuizResult, QuizSubmission,
    UpdateCourseRequest, UpdateLessonRequest, UserLearnStats, UserProgress,
    UserProgressResponse, AssignmentResponse,
};

pub mod creator {
    pub use crate::models::*;
    pub use crate::course::*;
    pub use crate::certification::*;
    pub use crate::gamification::*;
    pub use crate::routes_learn::configure_learn_api_routes;
}

pub fn configure_learn_ui_routes() -> axum::Router<()> {
    ui::configure_learn_ui_routes()
}
