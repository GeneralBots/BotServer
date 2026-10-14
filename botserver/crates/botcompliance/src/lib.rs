use axum::{
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use std::sync::Arc;

pub mod access_review;
pub mod audit;
pub mod backup_verification;
pub mod code_scanner;
pub mod dashboard;
pub mod evidence_collection;
pub mod framework_handlers;
pub mod handlers;
pub mod incident_response;
pub mod policy_checker;
pub mod risk_assessment;
pub mod schema;
pub mod soc2;
mod soc2_controls;
mod soc2_ops;
pub mod sop_middleware;
pub mod storage;
pub mod training_tracker;
pub mod types;
pub mod ui;
pub mod vulnerability_scanner;

pub use code_scanner::{
    CodeIssue, CodeScanner, ComplianceReporter, ComplianceScanResult, IssueSeverity, IssueType,
    ScanStats,
};

pub use storage::{
    DbAccessReview, DbAuditLog, DbComplianceCheck, DbComplianceControl,
    DbComplianceControlEvidence, DbComplianceFramework, DbComplianceIssue, DbEvidence,
    DbRiskAssessment, DbTrainingRecord,
};

pub use types::{
    AccessReview, ActionResult, AuditEventType, AuditLogEntry, ComplianceCheckResult,
    ComplianceFramework, ComplianceIssueResult, ComplianceReport, ComplianceStatus,
    CreateAuditLogRequest, CreateIssueRequest, CreateTrainingRequest, ListAuditLogsQuery,
    ListChecksQuery, ListIssuesQuery, PermissionReview, ReviewAction, ReviewStatus, Risk,
    RiskAssessment, RiskCategory, RiskStatus, RunCheckRequest, Severity, TrainingRecord,
    TrainingType, TreatmentStrategy, UpdateIssueRequest,
};

pub type DbPool = diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>;
pub type AppState = Arc<DbPool>;

#[derive(Debug, thiserror::Error)]
pub enum ComplianceError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for ComplianceError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        let (status, message) = match &self {
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            Self::Validation(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            Self::Database(msg) | Self::Internal(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg.clone())
            }
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

pub fn configure_compliance_routes() -> Router<AppState> {
    Router::new()
        .route("/api/compliance/checks", get(handlers::handle_list_checks).post(handlers::handle_run_check))
        .route("/api/compliance/checks/:check_id", get(handlers::handle_get_check))
        .route("/api/compliance/checks/:check_id/run", post(handlers::handle_run_check_by_id))
        .route("/api/compliance/issues", get(handlers::handle_list_issues).post(handlers::handle_create_issue))
        .route("/api/compliance/issues/:issue_id", put(handlers::handle_update_issue))
        .route("/api/compliance/audit", get(handlers::handle_list_audit_logs).post(handlers::handle_create_audit_log))
        .route("/api/compliance/audit-log", get(handlers::handle_list_audit_logs).post(handlers::handle_create_audit_log))
        .route("/api/compliance/training", get(handlers::handle_list_training).post(handlers::handle_create_training))
        .route("/api/compliance/risks", get(handlers::handle_list_risks))
        .route("/api/compliance/report", get(handlers::handle_get_report))
        .route("/api/compliance/evidence", post(handlers::handle_upload_evidence))
        // Frameworks CRUD + controls + evidence mapping
        .route("/api/compliance/frameworks", get(framework_handlers::handle_list_frameworks).post(framework_handlers::handle_create_framework))
        .route("/api/ui/compliance/framework/:name", get(framework_handlers::handle_framework_detail_page))
        .route("/api/compliance/frameworks/:framework_id", get(framework_handlers::handle_get_framework).put(framework_handlers::handle_update_framework))
        .route("/api/compliance/frameworks/:framework_id/archive", post(framework_handlers::handle_archive_framework))
        .route("/api/compliance/frameworks/:framework_id/export.csv", get(framework_handlers::handle_export_framework_csv))
        .route("/api/compliance/controls", post(framework_handlers::handle_create_control))
        .route("/api/compliance/controls/:control_id", put(framework_handlers::handle_update_control))
        .route("/api/compliance/evidence/attach", post(framework_handlers::handle_attach_evidence))
        .route("/api/compliance/evidence/:evidence_id/approve", post(framework_handlers::handle_approve_evidence))
        .route("/api/compliance/evidence/:evidence_id", delete(framework_handlers::handle_delete_evidence))
}

mod re_cache;
