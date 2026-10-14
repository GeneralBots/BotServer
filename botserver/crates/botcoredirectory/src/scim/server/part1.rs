use super::*;

pub fn configure_scim_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/scim/v2/Users", get(list_users).post(create_user))
        .route("/scim/v2/Users/:user_id", get(get_user).put(update_user).delete(delete_user))
        .route("/scim/v2/Users/:user_id/replace", post(replace_user))
        .route("/scim/v2/Groups", get(list_groups).post(create_group))
        .route("/scim/v2/Groups/:group_id", get(get_group).put(update_group).delete(delete_group))
        .route("/scim/v2/Groups/:group_id/replace", post(replace_group))
        .route("/scim/v2/Me", get(get_me))
        .route("/scim/v2/ServiceProviderConfig", get(service_provider_config))
        .route("/scim/v2/ResourceTypes", get(resource_types))
        .route("/scim/v2/Schemas", get(schemas))
}
pub(super) async fn service_provider_config() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:ServiceProviderConfig"],
        "patch": { "supported": true },
        "bulk": { "supported": false, "maxOperations": 0, "maxPayloadSize": 0 },
        "filter": { "supported": true, "maxResults": 100 },
        "changePassword": { "supported": false },
        "sort": { "supported": false },
        "etag": { "supported": false },
        "authenticationSchemes": [{
            "type": "oauthbearertoken",
            "name": "OAuth Bearer Token",
            "description": "Authentication scheme using the OAuth Bearer Token Standard",
            "specUri": "https://www.rfc-editor.org/info/rfc6750",
            "primary": true
        }]
    }))
}
pub(super) async fn resource_types() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:ResourceType"],
        "resources": [{
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:ResourceType"],
            "id": "User",
            "name": "User",
            "endpoint": "/Users",
            "description": "User account",
            "schema": "urn:ietf:params:scim:schemas:core:2.0:User"
        }, {
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:ResourceType"],
            "id": "Group",
            "name": "Group",
            "endpoint": "/Groups",
            "description": "Group of users",
            "schema": "urn:ietf:params:scim:schemas:core:2.0:Group"
        }]
    }))
}
pub(super) async fn schemas() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "schemas": ["urn:ietf:params:scim:schemas:core:2.0:Schema"],
        "resources": [{
            "id": "urn:ietf:params:scim:schemas:core:2.0:User",
            "name": "User",
            "description": "Core User Schema",
            "attributes": [{
                "name": "userName",
                "type": "string",
                "required": true,
                "multiValued": false,
                "description": "Unique identifier for the User"
            }]
        }, {
            "id": "urn:ietf:params:scim:schemas:core:2.0:Group",
            "name": "Group",
            "description": "Core Group Schema",
            "attributes": [{
                "name": "displayName",
                "type": "string",
                "required": true,
                "multiValued": false,
                "description": "A human-readable name for the Group"
            }]
        }]
    }))
}
pub(super) async fn get_me(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<ScimUser>, (StatusCode, Json<ScimError>)> {
    Err((StatusCode::NOT_IMPLEMENTED, Json(ScimError {
        schemas: vec!["urn:ietf:params:scim:api:messages:2.0:Error".to_string()],
        scim_type: None,
        detail: "GET /Me not supported - use specific user ID".to_string(),
        status: 501,
    })))
}
pub(super) async fn list_users(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ScimListResponse>, (StatusCode, Json<ScimError>)> {
    let start_index = params.get("startIndex")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1);
    let count = params.get("count")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(20);

    info!("SCIM GET /Users startIndex={} count={}", start_index, count);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ScimError {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:Error".to_string()],
            scim_type: None,
            detail: "No auth service".to_string(),
            status: 500,
        }))
    })?.lock().await;

    let filter = params.get("filter").map(|s| s.as_str());
    let users = match filter {
        Some(f) if f.starts_with("userName eq ") => {
            let query = f.trim_start_matches("userName eq ").trim_matches('"');
            auth_service.search_users(query).await.unwrap_or_default()
        }
        _ => {
            match auth_service.list_users(count as i64, (start_index - 1) as i64).await {
                Ok(data) => data
                    .as_array()
                    .cloned()
                    .unwrap_or_default(),
                Err(_) => vec![],
            }
        }
    };

    let total = users.len() as u32;
    let resources: Vec<serde_json::Value> = users.iter()
        .map(|u| serde_json::to_value(ScimUser::from_zitadel_user(u, vec![])).unwrap_or_default())
        .collect();

    Ok(Json(ScimListResponse {
        schemas: vec!["urn:ietf:params:scim:api:messages:2.0:ListResponse".to_string()],
        total_results: total,
        start_index,
        items_per_page: count,
        resources,
    }))
}
pub(super) async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Result<Json<ScimUser>, (StatusCode, Json<ScimError>)> {
    info!("SCIM GET /Users/{}", user_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ScimError {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:Error".to_string()],
            scim_type: None,
            detail: "No auth service".to_string(),
            status: 500,
        }))
    })?.lock().await;

    let user_data = auth_service.get_user(&user_id).await.map_err(|e| {
        (StatusCode::NOT_FOUND, Json(ScimError {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:Error".to_string()],
            scim_type: Some("invalidValue".to_string()),
            detail: format!("User not found: {}", e),
            status: 404,
        }))
    })?;

    let memberships = auth_service.get_user_memberships(&user_id, 0, 100).await
        .unwrap_or_default();
    let groups: Vec<ScimGroupRef> = memberships.get("result")
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| {
                    m.get("organizationId").and_then(|v| v.as_str()).map(|org_id| {
                        ScimGroupRef {
                            value: org_id.to_string(),
                            reference: Some(format!("/Groups/{}", org_id)),
                            display: None,
                        }
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(Json(ScimUser::from_zitadel_user(&user_data, groups)))
}
pub(super) async fn create_user(
    State(state): State<Arc<AppState>>,
    Json(scim_user): Json<ScimUser>,
) -> Result<(StatusCode, Json<ScimUser>), (StatusCode, Json<ScimError>)> {
    info!("SCIM POST /Users - creating: {}", scim_user.user_name);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ScimError {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:Error".to_string()],
            scim_type: None,
            detail: "No auth service".to_string(),
            status: 500,
        }))
    })?.lock().await;

    let body = scim_user.to_zitadel_json();
    let result = auth_service.http_post(
        format!("{}/v2/users", auth_service.api_url()),
        body,
    ).await.map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(ScimError {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:Error".to_string()],
            scim_type: None,
            detail: format!("Failed to create user: {}", e),
            status: 400,
        }))
    })?;

    let new_id = result.get("userId")
        .or_else(|| result.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let mut created = scim_user.clone();
    created.id = Some(new_id.clone());
    created.meta = Some(ScimMeta {
        resource_type: "User".to_string(),
        created: Some(chrono::Utc::now().to_rfc3339()),
        last_modified: Some(chrono::Utc::now().to_rfc3339()),
        location: Some(format!("/Users/{}", new_id)),
        version: None,
    });

    Ok((StatusCode::CREATED, Json(created)))
}
pub(super) async fn update_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
    Json(scim_user): Json<ScimUser>,
) -> Result<Json<ScimUser>, (StatusCode, Json<ScimError>)> {
    info!("SCIM PUT /Users/{}", user_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ScimError {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:Error".to_string()],
            scim_type: None,
            detail: "No auth service".to_string(),
            status: 500,
        }))
    })?.lock().await;

    let body = scim_user.to_zitadel_json();
    auth_service.http_patch(
        format!("{}/v2/users/{}", auth_service.api_url(), user_id),
        body,
    ).await.map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(ScimError {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:Error".to_string()],
            scim_type: None,
            detail: format!("Failed to update user: {}", e),
            status: 400,
        }))
    })?;

    let mut updated = scim_user;
    updated.id = Some(user_id.clone());
    Ok(Json(updated))
}
pub(super) async fn replace_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
    Json(scim_user): Json<ScimUser>,
) -> Result<Json<ScimUser>, (StatusCode, Json<ScimError>)> {
    update_user(State(state), Path(user_id), Json(scim_user)).await
}
