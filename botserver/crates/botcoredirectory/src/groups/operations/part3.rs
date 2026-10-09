use super::*;

pub async fn add_group_member(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<String>,
    Json(req): Json<AddMemberRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Adding user {} to group {}", req.user_id, group_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;


    let roles = req.roles.unwrap_or_else(|| vec!["ORG_USER".to_string()]);

    match auth_service.add_org_member(&group_id, &req.user_id, roles).await {
        Ok(_) => {
            info!(
                "User {} added to group {} successfully",
                req.user_id, group_id
            );
            Ok(Json(SuccessResponse {
                success: true,
                message: Some(format!(
                    "User {} added to group {} successfully",
                    req.user_id, group_id
                )),
                group_id: Some(group_id),
            }))
        }
        Err(e) => {
            log::error!("Failed to add member to group: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to add member to group".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}
pub async fn remove_group_member(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<String>,
    Json(req): Json<AddMemberRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Removing user {} from group {}", req.user_id, group_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;


    match auth_service.remove_org_member(&group_id, &req.user_id).await {
        Ok(_) => {
            info!(
                "User {} removed from group {} successfully",
                req.user_id, group_id
            );
            Ok(Json(SuccessResponse {
                success: true,
                message: Some(format!(
                    "User {} removed from group {} successfully",
                    req.user_id, group_id
                )),
                group_id: Some(group_id),
            }))
        }
        Err(e) => {
            log::error!("Failed to remove member from group: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to remove member from group".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}
