use super::*;

pub async fn create_user(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Creating user: {} ({})", req.username, req.email);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

    let user_id = match auth_service
        .create_user(&req.email, &req.first_name, &req.last_name, Some(&req.username))
        .await
    {
        Ok(id) => id,
        Err(e) => {
            log::error!("Failed to create user in Zitadel: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to create user".to_string(),
                    details: Some(e),
                }),
            ));
        }
    };

    if let Some(password) = &req.password {
        if let Err(e) = auth_service.set_user_password(&user_id, password).await {
            error!("Failed to set initial password for user {}: {}", user_id, e);
        } else {
            info!("Initial password set for user {}", user_id);
        }
    }

    if let Some(ref org_id) = req.organization_id {
        let roles = req.roles.clone().unwrap_or_else(|| vec!["user".to_string()]);

        if let Err(e) = auth_service.add_org_member(org_id, &user_id, roles.clone()).await {
            error!(
                "Failed to add user {} to organization {}: {}",
                user_id, org_id, e
            );
        } else {
            info!(
                "User {} added to organization {} with roles {:?}",
                user_id, org_id, roles
            );
        }
    }

    info!("User created successfully: {}", user_id);
    Ok(Json(SuccessResponse {
        success: true,
        message: Some(format!("User {} created successfully", req.username)),
        user_id: Some(user_id),
    }))
}
pub async fn update_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Updating user: {}", user_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

    let mut update_data = serde_json::Map::new();
    if let Some(username) = &req.username {
        update_data.insert("userName".to_string(), serde_json::json!(username));
    }
    if let Some(email) = &req.email {
        update_data.insert("email".to_string(), serde_json::json!(email));
    }
    if let Some(first_name) = &req.first_name {
        update_data.insert("firstName".to_string(), serde_json::json!(first_name));
    }
    if let Some(last_name) = &req.last_name {
        update_data.insert("lastName".to_string(), serde_json::json!(last_name));
    }
    if let Some(display_name) = &req.display_name {
        update_data.insert("displayName".to_string(), serde_json::json!(display_name));
    }
    if let Some(phone) = &req.phone {
        update_data.insert("phone".to_string(), serde_json::json!(phone));
    }

    if !update_data.is_empty() {
        match auth_service
            .http_patch(format!("{}/users/{}", auth_service.api_url(), user_id), serde_json::Value::Object(update_data))
            .await
        {
            Ok(_) => {
                info!("User {} profile updated successfully", user_id);
            }
            Err(e) => {
                log::error!("Failed to update user profile: {}", e);
            }
        }
    }

    if let Some(ref org_id) = req.organization_id {
        let roles = req.roles.clone().unwrap_or_else(|| vec!["user".to_string()]);

        if let Err(e) = auth_service.add_org_member(org_id, &user_id, roles.clone()).await {
            error!(
                "Failed to update user {} organization membership: {}",
                user_id, e
            );
        } else {
            info!(
                "User {} organization membership updated to {} with roles {:?}",
                user_id, org_id, roles
            );
        }
    }

    Ok(Json(SuccessResponse {
        success: true,
        message: Some(format!("User {} updated successfully", user_id)),
        user_id: Some(user_id),
    }))
}
pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Deleting user: {}", user_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

    match auth_service
        .http_delete(format!("{}/v2/users/{}", auth_service.api_url(), user_id))
        .await
    {
        Ok(_) => {
            info!("User {} deleted successfully", user_id);
            Ok(Json(SuccessResponse {
                success: true,
                message: Some(format!("User {} deleted successfully", user_id)),
                user_id: Some(user_id),
            }))
        }
        Err(e) => {
            log::error!("Failed to delete user: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to delete user".to_string(),
                    details: Some(e),
                }),
            ))
        }
    }
}
pub async fn list_users(
    State(state): State<Arc<AppState>>,
    Query(params): Query<UserQuery>,
) -> Result<Json<UserListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(20);

    info!("Listing users (page: {}, per_page: {})", page, per_page);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

    let users_result = if let Some(ref org_id) = params.organization_id {
        info!("Filtering users by organization: {}", org_id);
        auth_service.get_org_members(org_id).await
    } else if let Some(ref search_term) = params.search {
        info!("Searching users with term: {}", search_term);
        auth_service.search_users(search_term).await
    } else {
        let offset = (page - 1) * per_page;
        auth_service.list_users(per_page as i64, offset as i64).await.map(|v| v.as_array().cloned().unwrap_or_default())
    };

    match users_result {
        Ok(users_json) => {
            let users: Vec<UserResponse> = users_json
                .into_iter()
                .filter_map(|u| {
                    let id = u.get("userId").and_then(|v| v.as_str()).map(String::from)
                        .or_else(|| u.get("user_id").and_then(|v| v.as_str()).map(String::from))?;

                    let username = u.get("userName").and_then(|v| v.as_str())
                        .or_else(|| u.get("username").and_then(|v| v.as_str()))
                        .unwrap_or("unknown")
                        .to_string();

                    let email = u.get("preferredLoginName").and_then(|v| v.as_str())
                        .or_else(|| u.get("email").and_then(|v| v.as_str()))
                        .unwrap_or("unknown@example.com")
                        .to_string();

                    let first_name = u.get("profile")
                        .and_then(|p| p.get("givenName"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let last_name = u.get("profile")
                        .and_then(|p| p.get("familyName"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let display_name = u.get("profile")
                        .and_then(|p| p.get("displayName"))
                        .and_then(|v| v.as_str())
                        .map(String::from);

                    let state = u.get("state").and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();

                    let organization_id = u.get("orgId").and_then(|v| v.as_str())
                        .or_else(|| u.get("organization_id").and_then(|v| v.as_str()))
                        .map(String::from);

                    let roles = u.get("roles")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|r| r.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();

                    Some(UserResponse {
                        id,
                        username,
                        email,
                        first_name,
                        last_name,
                        display_name,
                        state,
                        organization_id,
                        roles,
                        created_at: None,
                        updated_at: None,
                    })
                })
                .collect();

            let total = users.len();
            info!("Found {} users", total);

            Ok(Json(UserListResponse {
                users,
                total,
                page,
                per_page,
            }))
        }
        Err(e) => {
            log::error!("Failed to list users: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to list users".to_string(),
                    details: Some(e.to_string()),
                }),
            ))
        }
    }
}
