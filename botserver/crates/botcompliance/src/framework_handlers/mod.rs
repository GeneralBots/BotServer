//! Compliance framework configuration: CRUD for frameworks, their control
//! catalogs and attached evidence, plus coverage computation and audit-ready
//! CSV export. Every mutation is recorded in `compliance_audit_log`.

pub(crate) use axum::{ extract::{Path, Query, State}, response::{Html, IntoResponse}, Json, };
pub(crate) use chrono::Utc;
pub(crate) use diesel::prelude::*;
pub(crate) use serde::Deserialize;
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;
pub(crate) use crate::schema::{ compliance_audit_log, compliance_control_evidence, compliance_controls, compliance_frameworks, };
pub(crate) use crate::storage::{ DbComplianceControl, DbComplianceControlEvidence, DbComplianceFramework, };
pub(crate) use crate::ComplianceError;

const BRANCH_ID_PLACEHOLDER: Uuid = Uuid::nil();

#[derive(Debug, Deserialize)]
pub struct CreateFrameworkRequest {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub framework_key: Option<String>,
    pub created_by: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFrameworkRequest {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateControlRequest {
    pub framework_id: Uuid,
    pub control_id: String,
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub is_mandatory: Option<bool>,
    pub created_by: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateControlRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub is_mandatory: Option<bool>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AttachEvidenceRequest {
    pub control_id: Uuid,
    pub file_path: String,
    pub description: Option<String>,
    pub evidence_type: Option<String>,
    pub owner_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct ListFrameworksQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
}

mod part1;
mod part2;
mod part3;

pub use part1::*;
pub use part2::*;
pub use part3::*;
