use super::*;

pub async fn create_group(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateGroupRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Creating group: {}", req.name);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

    let metadata_key = format!("group_{}", Uuid::new_v4());
    let metadata_value = serde_json::json!({
        "name": req.name,
        "description": req.description,
        "members": req.members.unwrap_or_default(),
        "created_at": chrono::Utc::now().to_rfc3339()
    })
    .to_string();

    let body = serde_json::json!({
        "key": metadata_key,
        "value": metadata_value
    });

    match auth_service
        .http_post(format!("{}/metadata/organization", auth_service.api_url()), body)
        .await
    {
        Ok(_) => {
            info!("Group created successfully: {}", metadata_key);
            Ok(Json(SuccessResponse {
                success: true,
                message: Some(format!("Group '{}' created successfully", req.name)),
                group_id: Some(metadata_key),
            }))
        }
        Err(e) => {
            log::error!("Error creating group: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: botsecurity::error_sanitizer::log_and_sanitize_str(&e, "botcoredirectory::groups::operations::part1", None).message,
                    details: None,
                }),
            ))
        }
    }
}
pub async fn update_group(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<String>,
    Json(req): Json<UpdateGroupRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Updating group: {}", group_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;


    let mut update_data = serde_json::Map::new();
    if let Some(name) = &req.name {
        update_data.insert("name".to_string(), serde_json::json!(name));
    }
    if let Some(description) = &req.description {
        update_data.insert("description".to_string(), serde_json::json!(description));
    }
    if let Some(members) = &req.members {
        update_data.insert("members".to_string(), serde_json::json!(members));
    }
    update_data.insert(
        "updated_at".to_string(),
        serde_json::json!(chrono::Utc::now().to_rfc3339()),
    );


    let body = serde_json::json!({
        "value": serde_json::Value::Object(update_data).to_string()
    });

    match auth_service
        .http_put(format!(
            "{}/metadata/organization/{}",
            auth_service.api_url(),
            group_id
        ), body)
        .await
    {
        Ok(_) => {
            info!("Group updated successfully: {}", group_id);
            Ok(Json(SuccessResponse {
                success: true,
                message: Some(format!("Group '{}' updated successfully", group_id)),
                group_id: Some(group_id),
            }))
        }
        Err(e) => {
            log::error!("Error updating group: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: botsecurity::error_sanitizer::log_and_sanitize_str(&e, "botcoredirectory::groups::operations::part1", None).message,
                    details: None,
                }),
            ))
        }
    }
}
pub async fn delete_group(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<String>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Deleting group: {}", group_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;


    let delete_url = format!("{}/metadata/organization/{}", auth_service.api_url(), group_id);
    // `http_delete` already performs the request and returns the parsed body.
    match auth_service.http_delete(delete_url).await {
        Ok(_) => {
            info!("Group {} deleted successfully", group_id);
            Ok(Json(SuccessResponse {
                success: true,
                message: Some(format!("Group {} deleted successfully", group_id)),
                group_id: Some(group_id),
            }))
        }
        Err(e) => {
            log::error!("Failed to delete group: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to delete group".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}
pub async fn list_groups(
    State(state): State<Arc<AppState>>,
    Query(params): Query<GroupQuery>,
) -> Result<Json<GroupListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(20);

    info!("Listing groups (page: {}, per_page: {})", page, per_page);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

    match auth_service
        .http_get(format!("{}/metadata/organization?limit={}&offset={}", auth_service.api_url(), per_page, (page - 1) * per_page))
        .await
    {
        Ok(data) => {
            let metadata: Vec<serde_json::Value> = data.as_array()
                .cloned()
                .unwrap_or_default();

            let groups: Vec<GroupInfo> = metadata
                .iter()
                .filter_map(|item| {
                    let key = item.get("key").and_then(|k| k.as_str())?;
                    if !key.starts_with("group_") {
                        return None;
                    }
                    let value_str = item.get("value").and_then(|v| v.as_str())?;
                    let group_data = serde_json::from_str::<serde_json::Value>(value_str).ok()?;
                    Some(GroupInfo {
                        id: key.to_string(),
                        name: group_data.get("name").and_then(|n| n.as_str()).unwrap_or("Unknown").to_string(),
                        description: group_data.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()),
                        member_count: group_data.get("members").and_then(|m| m.as_array()).map(|a| a.len()).unwrap_or(0),
                    })
                })
                .collect();

            info!("Found {} groups", groups.len());
            let total = groups.len();
            Ok(Json(GroupListResponse {
                groups,
                total,
                page,
                per_page,
            }))
        }
        Err(e) => {
            log::error!("Failed to list groups: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to list groups".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}
pub async fn update_group_member_roles(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<String>,
    Json(req): Json<UpdateMemberRolesRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Updating roles for user {} in group {}", req.user_id, group_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

    let _ = auth_service.remove_org_member(&group_id, &req.user_id).await;

    match auth_service.add_org_member(&group_id, &req.user_id, req.roles.clone()).await {
        Ok(_) => {
            info!("Roles updated for user {} in group {}", req.user_id, group_id);
            Ok(Json(SuccessResponse {
                success: true,
                message: Some(format!("Roles updated for user {} in group {}", req.user_id, group_id)),
                group_id: Some(group_id),
            }))
        }
        Err(e) => {
            log::error!("Failed to update roles: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to update member roles".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}
pub async fn get_group_settings(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    info!("Getting settings for group: {}", group_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

    match auth_service
        .http_get(format!("{}/metadata/organization/{}", auth_service.api_url(), group_id))
        .await
    {
        Ok(metadata) => Ok(Json(metadata)),
        Err(e) => {
            log::error!("Error getting group settings: {}", e);
            Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Group not found".to_string(),
                    details: None,
                }),
            ))
        }
    }
}
pub async fn get_group_permissions(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    info!("Getting permissions for group: {}", group_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

    // `http_get` already performs the request and returns the parsed JSON body.
    let metadata = auth_service
        .http_get(format!("{}/metadata/organization/{}", auth_service.api_url(), group_id))
        .await
        .map_err(|e| {
            log::error!("Error getting group permissions: {}", e);
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Group not found".to_string(),
                    details: Some(e.to_string()),
                }),
            )
        })?;

    let permissions = metadata
        .get("value")
        .and_then(|v| v.as_str())
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        .and_then(|group_data| group_data.get("permissions").cloned())
        .and_then(|p| p.as_array().map(|a| a.clone()))
        .unwrap_or_default();

    Ok(Json(serde_json::json!({
        "group_id": group_id,
        "permissions": permissions,
        "inherited": true,
        "source": "organization"
    })))
}
