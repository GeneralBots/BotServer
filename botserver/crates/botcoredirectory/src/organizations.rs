use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use log::info;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use botcore::shared::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CreateOrganizationRequest {
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OrganizationInfo {
    pub id: String,
    pub name: String,
    pub primary_domain: Option<String>,
    pub state: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OrganizationListResponse {
    pub organizations: Vec<OrganizationInfo>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: Option<String>,
    pub organization_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Option<String>,
}

pub async fn list_organizations(
    State(state): State<Arc<AppState>>,
) -> Result<Json<OrganizationListResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Listing organizations");

    let auth = state.auth_service.as_ref().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "No auth service".to_string(),
                details: None,
            }),
        )
    })?;

    let auth_service = auth.lock().await;

    match auth_service.list_organizations().await {
        Ok(data) => {
            let orgs = data
                .as_array()
                .cloned()
                .unwrap_or_default();

            let organizations: Vec<OrganizationInfo> = orgs
                .iter()
                .filter_map(|org| {
                    let id = org
                        .get("orgId")
                        .or_else(|| org.get("id"))
                        .and_then(|v| v.as_str())?
                        .to_string();
                    let name = org
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    let primary_domain = org
                        .get("primaryDomain")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let state_val = org
                        .get("state")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    Some(OrganizationInfo {
                        id,
                        name,
                        primary_domain,
                        state: state_val,
                    })
                })
                .collect();

            let total = organizations.len();
            info!("Found {} organizations", total);
            Ok(Json(OrganizationListResponse {
                organizations,
                total,
            }))
        }
        Err(e) => {
            log::error!("Failed to list organizations: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to list organizations".to_string(),
                    details: None,
                }),
            ))
        }
    }
}

pub async fn create_organization(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateOrganizationRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Creating organization: {}", req.name);

    let auth = state.auth_service.as_ref().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "No auth service".to_string(),
                details: None,
            }),
        )
    })?;

    let auth_service = auth.lock().await;

    match auth_service.create_organization(&req.name).await {
        Ok(org_id) => {
            info!("Organization created: {} ({})", req.name, org_id);
            Ok(Json(SuccessResponse {
                success: true,
                message: Some(format!("Organization '{}' created successfully", req.name)),
                organization_id: Some(org_id),
            }))
        }
        Err(e) => {
            log::error!("Failed to create organization: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to create organization".to_string(),
                    details: None,
                }),
            ))
        }
    }
}

pub async fn get_organization(
    State(state): State<Arc<AppState>>,
    Path(org_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    info!("Getting organization: {}", org_id);

    let auth = state.auth_service.as_ref().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "No auth service".to_string(),
                details: None,
            }),
        )
    })?;

    let auth_service = auth.lock().await;

    match auth_service.get_organization(&org_id).await {
        Ok(data) => Ok(Json(data)),
        Err(e) => {
            log::error!("Failed to get organization: {}", e);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Organization not found".to_string(),
                    details: None,
                }),
            ))
        }
    }
}
