pub(crate) use axum::{ extract::{Path, Query, State}, http::StatusCode, response::Json, routing::{delete, get, post}, Router, };
pub(crate) use chrono::{DateTime, Duration, Utc};
pub(crate) use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Declined,
    Expired,
    Revoked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InvitationRole {
    Owner,
    Admin,
    Manager,
    Member,
    Viewer,
    Guest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationInvitation {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub email: String,
    pub role: InvitationRole,
    pub groups: Vec<String>,
    pub invited_by: Uuid,
    pub invited_by_name: String,
    pub status: InvitationStatus,
    pub token: String,
    pub message: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub accepted_by: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInvitationRequest {
    pub email: String,
    pub role: String,
    #[serde(default)]
    pub groups: Vec<String>,
    pub message: Option<String>,
    pub expires_in_days: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct BulkInviteRequest {
    pub emails: Vec<String>,
    pub role: String,
    #[serde(default)]
    pub groups: Vec<String>,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AcceptInvitationRequest {
    pub token: String,
    pub user_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct ResendInvitationRequest {
    pub extend_expiry: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ListInvitationsQuery {
    pub status: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InvitationResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub organization_name: String,
    pub email: String,
    pub role: String,
    pub groups: Vec<String>,
    pub invited_by_name: String,
    pub status: String,
    pub message: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub is_expired: bool,
}

#[derive(Debug, Serialize)]
pub struct BulkInviteResponse {
    pub successful: Vec<InvitationResponse>,
    pub failed: Vec<BulkInviteError>,
}

#[derive(Debug, Serialize)]
pub struct BulkInviteError {
    pub email: String,
    pub error: String,
}

#[derive(Debug, Serialize)]
pub struct InvitationListResponse {
    pub invitations: Vec<InvitationResponse>,
    pub total: u32,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

#[derive(Debug, Serialize)]
pub struct AcceptInvitationResponse {
    pub success: bool,
    pub organization_id: Uuid,
    pub organization_name: String,
    pub role: String,
    pub message: String,
}

pub struct InvitationService {
    pool: DbPool,
}

/// Row shape of the persisted `organization_invitations` table (raw SQL —
/// botcore does not own the diesel schema for this table).
#[derive(diesel::QueryableByName, Debug, Clone)]
struct InvitationRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    org_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    email: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    role: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    status: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    message: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    invited_by: Uuid,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    invited_by_name: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    token: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Jsonb)]
    groups: serde_json::Value,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    created_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
    updated_at: Option<DateTime<Utc>>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
    expires_at: Option<DateTime<Utc>>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
    accepted_at: Option<DateTime<Utc>>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
    accepted_by: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct CreateInvitationParams<'a> {
    pub organization_id: Uuid,
    pub organization_name: &'a str,
    pub email: &'a str,
    pub role: InvitationRole,
    pub groups: Vec<String>,
    pub invited_by: Uuid,
    pub invited_by_name: &'a str,
    pub message: Option<String>,
    pub expires_in_days: i64,
}

pub struct BulkInviteParams<'a> {
    pub organization_id: Uuid,
    pub organization_name: &'a str,
    pub emails: Vec<String>,
    pub role: InvitationRole,
    pub groups: Vec<String>,
    pub invited_by: Uuid,
    pub invited_by_name: &'a str,
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a DB-backed service for tests. Requires `DATABASE_URL` (the
    /// `organization_invitations` table must exist); returns `None` when the
    /// env var is absent so unit runs without a database are skipped.
    fn test_service() -> Option<InvitationService> {
        use diesel::r2d2::{ConnectionManager, Pool};
        let url = std::env::var("DATABASE_URL").ok()?;
        let manager = ConnectionManager::<diesel::PgConnection>::new(url);
        // The pool type has to be named explicitly: `Pool::new` alone leaves
        // the connection manager's parameter unconstrained.
        let pool: DbPool = Pool::builder().max_size(2).build(manager).ok()?;
        Some(InvitationService::new(pool))
    }

    #[tokio::test]
    async fn test_create_invitation() {
        let Some(service) = test_service() else { return; };
        let org_id = Uuid::new_v4();
 
        let params = crate::organization_invitations::CreateInvitationParams {
            organization_id: org_id,
            organization_name: "Test Org",
            email: "test@example.com",
            role: InvitationRole::Member,
            groups: vec![],
            ..Default::default()
        };

        let result = service.create_invitation(params).await;

        assert!(result.is_ok());
        let invitation = result.unwrap();
        assert_eq!(invitation.email, "test@example.com");
        assert_eq!(invitation.status, InvitationStatus::Pending);
    }

    #[tokio::test]
    async fn test_duplicate_invitation() {
        let Some(service) = test_service() else { return; };
        let org_id = Uuid::new_v4();
 
        let params = crate::organization_invitations::CreateInvitationParams {
            organization_id: org_id,
            organization_name: "Test Org",
            email: "test@example.com",
            role: InvitationRole::Member,
            groups: vec![],
            ..Default::default()
        };

        let first_result = service.create_invitation(params.clone()).await;

        assert!(first_result.is_ok());

        let second_result = service.create_invitation(params).await;

        assert!(second_result.is_err());
        assert_eq!(
            second_result.unwrap_err(),
            "An invitation already exists for this email"
        );
    }

    #[tokio::test]
    async fn test_accept_invitation() {
        let Some(service) = test_service() else { return; };
        let org_id = Uuid::new_v4();
        let invited_by = Uuid::new_v4();
        let user_id = Uuid::new_v4();
 
        let params = crate::organization_invitations::CreateInvitationParams {
            organization_id: org_id,
            organization_name: "Test Org",
            email: "test@example.com",
            role: InvitationRole::Member,
            groups: vec![],
            invited_by,
            invited_by_name: "Admin",
            message: None,
            expires_in_days: 7,
        };
        let invitation = service.create_invitation(params).await.unwrap();

        let result = service.accept_invitation(&invitation.token, user_id).await;
        assert!(result.is_ok());

        result.unwrap();
        let updated = service.get_invitation(invitation.id).await.unwrap();
        assert_eq!(updated.status, InvitationStatus::Accepted);
        assert!(updated.accepted_at.is_some());
    }
}

use std::sync::Arc;
use uuid::Uuid;
use crate::shared::state::AppState;
use crate::shared::utils::DbPool;

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;
mod part8;

pub use part7::*;
use part8::*;
