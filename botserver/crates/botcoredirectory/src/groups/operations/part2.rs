use super::*;

pub async fn get_group_analytics(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    info!("Getting analytics for group: {}", group_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

    // `http_get` already performs the request and returns the parsed JSON body.
    let metadata = auth_service
        .http_get(format!("{}/metadata/organization/{}", auth_service.api_url(), group_id))
        .await
        .map_err(|e| {
            log::error!("Error getting group analytics: {}", e);
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Group not found".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    let group_data = metadata
        .get("value")
        .and_then(|v| v.as_str())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());

    let member_count = group_data
        .as_ref()
        .and_then(|g| g.get("members"))
        .and_then(|m| m.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    // "active_users" would require per-user session tracking per group;
    // "kbs_shared" would require cross-referencing KB grants. Both are
    // reported as 0 (no tracked state) rather than fabricated numbers.
    let analytics = serde_json::json!({
        "group_id": group_id,
        "member_count": member_count,
        "active_users": 0,
        "kbs_shared": 0
    });
    Ok(Json(analytics))
}
pub async fn request_join_group(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<String>,
    Json(req): Json<AddMemberRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Join request for group {} from user {}", group_id, req.user_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

    let roles = req.roles.unwrap_or_else(|| vec!["ORG_USER".to_string()]);

    match auth_service.add_org_member(&group_id, &req.user_id, roles).await {
        Ok(_) => {
            info!("User {} joined group {}", req.user_id, group_id);
            Ok(Json(SuccessResponse {
                success: true,
                message: Some(format!("User {} joined group {}", req.user_id, group_id)),
                group_id: Some(group_id),
            }))
        }
        Err(e) => {
            log::error!("Failed to process join request: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to process join request".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}
pub async fn approve_join_request(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<String>,
    Json(req): Json<AddMemberRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Approving join request for user {} in group {}", req.user_id, group_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

    let roles = req.roles.unwrap_or_else(|| vec!["ORG_USER".to_string()]);

    match auth_service.add_org_member(&group_id, &req.user_id, roles).await {
        Ok(_) => {
            info!("User {} approved to join group {}", req.user_id, group_id);
            Ok(Json(SuccessResponse {
                success: true,
                message: Some(format!("User {} approved to join group {}", req.user_id, group_id)),
                group_id: Some(group_id),
            }))
        }
        Err(e) => {
            log::error!("Failed to approve join request: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to approve join request".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}
pub async fn reject_join_request(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<String>,
    Json(req): Json<AddMemberRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Rejecting join request for user {} in group {}", req.user_id, group_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

    match auth_service.remove_org_member(&group_id, &req.user_id).await {
        Ok(_) => {
            info!("User {} rejected from group {}", req.user_id, group_id);
            Ok(Json(SuccessResponse {
                success: true,
                message: Some(format!("User {} rejected from group {}", req.user_id, group_id)),
                group_id: Some(group_id),
            }))
        }
        Err(e) => {
            log::error!("Failed to reject join request: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to reject join request".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}
pub async fn send_group_invite(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<String>,
    Json(req): Json<AddMemberRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Sending invite for group {} to user {}", group_id, req.user_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

    let roles = req.roles.unwrap_or_else(|| vec!["ORG_USER".to_string()]);

    match auth_service.add_org_member(&group_id, &req.user_id, roles).await {
        Ok(_) => {
            info!("User {} invited to group {}", req.user_id, group_id);
            Ok(Json(SuccessResponse {
                success: true,
                message: Some(format!("User {} invited to group {}", req.user_id, group_id)),
                group_id: Some(group_id),
            }))
        }
        Err(e) => {
            log::error!("Failed to send invite: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to send invite".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}
pub async fn list_group_invites(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    info!("Listing invites for group: {}", group_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

    match auth_service
        .http_get(format!("{}/metadata/organization/{}", auth_service.api_url(), group_id))
        .await
    {
        Ok(metadata) => {
            let invites = serde_json::json!({
                "group_id": group_id,
                "invites": [],
                "metadata": metadata
            });
            Ok(Json(invites))
        }
        Err(e) => {
            log::error!("Error listing invites: {}", e);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Group not found".to_string(),
                    details: Some(e),
                }),
            ))
        }
    }
}
pub async fn get_group_members(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<String>,
) -> Result<Json<Vec<GroupMemberResponse>>, (StatusCode, Json<ErrorResponse>)> {
    info!("Getting members for group: {}", group_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;


    match auth_service
        .http_get(format!(
            "{}/metadata/organization/{}",
            auth_service.api_url(),
            group_id
        ))
        .await
    {
        Ok(metadata) => {
            if let Some(value_str) = metadata.get("value").and_then(|v| v.as_str()) {
                if let Ok(group_data) = serde_json::from_str::<serde_json::Value>(value_str) {
                    if let Some(member_ids) = group_data.get("members").and_then(|m| m.as_array()) {

                        let mut members = Vec::new();

                        for member_id in member_ids {
                            if let Some(user_id) = member_id.as_str() {
                                if let Ok(user_data) = auth_service
                                    .get_user(user_id)
                                    .await
                                {
                                    members.push(GroupMemberResponse {
                                        user_id: user_id.to_string(),
                                        username: user_data
                                            .get("userName")
                                            .and_then(|u| u.as_str())
                                            .map(|s| s.to_string()),
                                        email: user_data
                                            .get("profile")
                                            .and_then(|p| p.get("email"))
                                            .and_then(|e| e.as_str())
                                            .map(|s| s.to_string()),
                                        roles: vec![],
                                    });
                                }
                            }
                        }

                        info!("Found {} members in group {}", members.len(), group_id);
                        return Ok(Json(members));
                    }
                }
            }

            info!("Group {} has no members", group_id);
            Ok(Json(vec![]))
        }
        Err(e) => {
            log::error!("Error getting group members: {}", e);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Group not found".to_string(),
                    details: Some(e),
                }),
            ))
        }
    }
}
