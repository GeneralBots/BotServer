use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::schema::{
    compliance_access_reviews, compliance_audit_log, compliance_checks, compliance_control_evidence,
    compliance_controls, compliance_evidence, compliance_frameworks, compliance_issues,
    compliance_risk_assessments, compliance_training_records,
};

use crate::types::{
    ActionResult, AuditEventType, AuditLogEntry, ComplianceCheckResult, ComplianceFramework,
    ComplianceIssueResult, ComplianceStatus, Severity,
};

#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = compliance_checks)]
pub struct DbComplianceCheck {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub status: Option<String>,
    pub checked_at: Option<DateTime<Utc>>,
    pub checked_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub org_id: Option<Uuid>,
    pub bot_id: Option<Uuid>,
    pub check_type: String,
    pub target_type: Option<String>,
    pub target_id: Option<Uuid>,
    pub result: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = compliance_issues)]
pub struct DbComplianceIssue {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub check_id: Option<Uuid>,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub remediation: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub assigned_to: Option<Uuid>,
    pub status: String,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<Uuid>,
    pub resolution_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub branch_id: Uuid,
}

#[derive(Debug, Clone, Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = compliance_audit_log)]
pub struct DbAuditLog {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub event_type: String,
    pub user_id: Option<Uuid>,
    pub resource_type: String,
    pub resource_id: String,
    pub action: String,
    pub result: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub branch_id: Uuid,
}

#[derive(Debug, Clone, Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = compliance_evidence)]
pub struct DbEvidence {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub check_id: Option<Uuid>,
    pub issue_id: Option<Uuid>,
    pub evidence_type: String,
    pub title: String,
    pub description: Option<String>,
    pub file_url: Option<String>,
    pub file_name: Option<String>,
    pub file_size: Option<i32>,
    pub mime_type: Option<String>,
    pub metadata: serde_json::Value,
    pub collected_at: DateTime<Utc>,
    pub collected_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub branch_id: Uuid,
}

#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = compliance_risk_assessments)]
pub struct DbRiskAssessment {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub title: String,
    pub assessor_id: Uuid,
    pub methodology: String,
    pub overall_risk_score: BigDecimal,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub next_review_date: Option<NaiveDate>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub branch_id: Uuid,
}

#[derive(Debug, Clone, Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = compliance_training_records)]
pub struct DbTrainingRecord {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub user_id: Uuid,
    pub training_type: String,
    pub training_name: String,
    pub provider: Option<String>,
    pub score: Option<i32>,
    pub passed: bool,
    pub completion_date: DateTime<Utc>,
    pub valid_until: Option<DateTime<Utc>>,
    pub certificate_url: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub branch_id: Uuid,
}

#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = compliance_access_reviews)]
pub struct DbAccessReview {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub user_id: Uuid,
    pub reviewer_id: Uuid,
    pub review_date: DateTime<Utc>,
    pub permissions_reviewed: serde_json::Value,
    pub anomalies: serde_json::Value,
    pub recommendations: serde_json::Value,
    pub status: String,
    pub approved_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub branch_id: Uuid,
}

pub fn db_check_to_result(
    db: DbComplianceCheck,
    issues: Vec<ComplianceIssueResult>,
) -> ComplianceCheckResult {
    let framework: ComplianceFramework = db.check_type.parse().unwrap_or(ComplianceFramework::Gdpr);
    let status: ComplianceStatus = db
        .status
        .as_deref()
        .unwrap_or("in_progress")
        .parse()
        .unwrap_or(ComplianceStatus::InProgress);
    let score: f64 = db
        .result
        .as_ref()
        .and_then(|r| r.get("score").and_then(|v| v.as_f64()))
        .unwrap_or(0.0);
    let checked_at = db.checked_at.unwrap_or(db.created_at);

    ComplianceCheckResult {
        id: db.id,
        framework,
        control_id: String::new(),
        control_name: String::new(),
        status,
        score,
        checked_at,
        checked_by: db.checked_by,
        issues,
        evidence: Vec::new(),
        notes: None,
    }
}

pub fn db_issue_to_result(db: DbComplianceIssue) -> ComplianceIssueResult {
    let severity: Severity = db.severity.parse().unwrap_or(Severity::Medium);
    let status = db.status;

    ComplianceIssueResult {
        id: db.id,
        severity,
        title: db.title,
        description: db.description,
        remediation: db.remediation,
        due_date: db.due_date,
        assigned_to: db.assigned_to,
        status,
    }
}

pub fn db_audit_to_entry(db: DbAuditLog) -> AuditLogEntry {
    let event_type: AuditEventType = db.event_type.parse().unwrap_or(AuditEventType::Access);
    let result: ActionResult = db.result.parse().unwrap_or(ActionResult::Success);
    let metadata: HashMap<String, String> =
        serde_json::from_value(db.metadata).unwrap_or_default();

    AuditLogEntry {
        id: db.id,
        timestamp: db.created_at,
        event_type,
        user_id: db.user_id,
        resource_type: db.resource_type,
        resource_id: db.resource_id,
        action: db.action,
        result,
        ip_address: db.ip_address,
        user_agent: db.user_agent,
        metadata,
    }
}

#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = compliance_frameworks)]
pub struct DbComplianceFramework {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub framework_key: String,
    pub status: String,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = compliance_controls)]
pub struct DbComplianceControl {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub framework_id: Uuid,
    pub control_id: String,
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub is_mandatory: bool,
    pub version: String,
    pub status: String,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = compliance_control_evidence)]
pub struct DbComplianceControlEvidence {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub control_id: Uuid,
    pub file_path: String,
    pub description: Option<String>,
    pub evidence_type: String,
    pub status: String,
    pub owner_id: Option<Uuid>,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub fn db_risk_assessment_to_json(db: DbRiskAssessment) -> serde_json::Value {
    serde_json::json!({
        "id": db.id,
        "title": db.title,
        "assessor_id": db.assessor_id,
        "methodology": db.methodology,
        "overall_risk_score": db.overall_risk_score,
        "status": db.status,
        "started_at": db.started_at,
        "completed_at": db.completed_at,
        "next_review_date": db.next_review_date,
        "notes": db.notes,
    })
}
