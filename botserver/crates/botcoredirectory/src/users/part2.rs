use super::*;

pub async fn get_user_profile(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Result<Json<UserResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Getting profile for user: {}", user_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

    match auth_service.get_user(&user_id).await {
        Ok(user_data) => {
            let id = user_data.get("id").and_then(|v| v.as_str())
                .unwrap_or(&user_id)
                .to_string();

            let username = user_data.get("username").and_then(|v| v.as_str())
                .or_else(|| user_data.get("userName").and_then(|v| v.as_str()))
                .unwrap_or("unknown")
                .to_string();

            let email = user_data.get("preferredLoginName").and_then(|v| v.as_str())
                .or_else(|| user_data.get("email").and_then(|v| v.as_str()))
                .unwrap_or("unknown@example.com")
                .to_string();

            let first_name = user_data.get("profile")
                .and_then(|p| p.get("givenName"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let last_name = user_data.get("profile")
                .and_then(|p| p.get("familyName"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let display_name = user_data.get("profile")
                .and_then(|p| p.get("displayName"))
                .and_then(|v| v.as_str())
                .map(String::from);

            let state = user_data.get("state").and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let organization_id = user_data.get("orgId").and_then(|v| v.as_str())
                .map(String::from);

            let roles = user_data.get("roles")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|r| r.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            let user = UserResponse {
                id,
                username: username.clone(),
                email,
                first_name,
                last_name,
                display_name,
                state,
                organization_id,
                roles,
                created_at: None,
                updated_at: None,
            };

            info!("User profile retrieved: {}", username);
            Ok(Json(user))
        }
        Err(e) => {
            log::error!("Failed to get user profile: {}", e);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "User not found".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}
pub async fn assign_organization(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
    Json(req): Json<AssignOrganizationRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "Assigning user {} to organization {}",
        user_id, req.organization_id
    );

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

    let roles = req.roles.unwrap_or_else(|| vec!["user".to_string()]);

    match auth_service
        .add_org_member(&req.organization_id, &user_id, roles.clone())
        .await
    {
        Ok(()) => {
            info!(
                "User {} assigned to organization {} with roles {:?}",
                user_id, req.organization_id, roles
            );
            Ok(Json(SuccessResponse {
                success: true,
                message: Some(format!(
                    "User assigned to organization {} with roles {:?}",
                    req.organization_id, roles
                )),
                user_id: Some(user_id),
            }))
        }
        Err(e) => {
            log::error!("Failed to assign user to organization: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to assign user to organization".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}
pub async fn remove_from_organization(
    State(state): State<Arc<AppState>>,
    Path((user_id, org_id)): Path<(String, String)>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Removing user {} from organization {}", user_id, org_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

    match auth_service.remove_org_member(&org_id, &user_id).await {
        Ok(()) => {
            info!("User {} removed from organization {}", user_id, org_id);
            Ok(Json(SuccessResponse {
                success: true,
                message: Some(format!("User removed from organization {}", org_id)),
                user_id: Some(user_id),
            }))
        }
        Err(e) => {
            log::error!("Failed to remove user from organization: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to remove user from organization".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}
pub async fn get_user_memberships(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    info!("Getting memberships for user: {}", user_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

    match auth_service.get_user_memberships(&user_id, 0, 100).await {
        Ok(memberships) => {
            info!("Retrieved memberships for user {}", user_id);
            Ok(Json(memberships))
        }
        Err(e) => {
            log::error!("Failed to get user memberships: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to get user memberships".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}
pub async fn update_user_roles(
    State(state): State<Arc<AppState>>,
    Path((user_id, org_id)): Path<(String, String)>,
    Json(req): Json<UpdateRolesRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "Updating roles for user {} in organization {}: {:?}",
        user_id, org_id, req.roles
    );

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

    if let Err(e) = auth_service.remove_org_member(&org_id, &user_id).await {
        log::error!("Failed to remove existing membership: {}", e);
    }

    match auth_service
        .add_org_member(&org_id, &user_id, req.roles.clone())
        .await
    {
        Ok(()) => {
            info!(
                "User {} roles updated in organization {}: {:?}",
                user_id, org_id, req.roles
            );
            Ok(Json(SuccessResponse {
                success: true,
                message: Some(format!("User roles updated to {:?}", req.roles)),
                user_id: Some(user_id),
            }))
        }
        Err(e) => {
            log::error!("Failed to update user roles: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to update user roles".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}
