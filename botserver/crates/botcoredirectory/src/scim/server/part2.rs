use super::*;

pub(super) async fn delete_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ScimError>)> {
    info!("SCIM DELETE /Users/{}", user_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ScimError {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:Error".to_string()],
            scim_type: None,
            detail: "No auth service".to_string(),
            status: 500,
        }))
    })?.lock().await;

    auth_service.http_delete(
        format!("{}/v2/users/{}", auth_service.api_url(), user_id),
    ).await.map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(ScimError {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:Error".to_string()],
            scim_type: None,
            detail: format!("Failed to delete user: {}", e),
            status: 400,
        }))
    })?;

    Ok(StatusCode::NO_CONTENT)
}
pub(super) async fn list_groups(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ScimListResponse>, (StatusCode, Json<ScimError>)> {
    let start_index = params.get("startIndex")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1);
    let count = params.get("count")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(20);

    info!("SCIM GET /Groups startIndex={} count={}", start_index, count);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ScimError {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:Error".to_string()],
            scim_type: None,
            detail: "No auth service".to_string(),
            status: 500,
        }))
    })?.lock().await;

    let data = auth_service.http_get(
        format!("{}/metadata/organization?limit={}&offset={}", auth_service.api_url(), count, start_index - 1),
    ).await.unwrap_or_default();

    let metadata = data.as_array().cloned().unwrap_or_default();
    let groups: Vec<serde_json::Value> = metadata.iter()
        .filter_map(|item| {
            let key = item.get("key")?.as_str()?;
            if !key.starts_with("group_") { return None; }
            let value_str = item.get("value")?.as_str()?;
            let value: serde_json::Value = serde_json::from_str(value_str).ok()?;
            let display_name = value.get("name")?.as_str()?.to_string();
            let member_ids: Vec<String> = value.get("members")
                .and_then(|m| m.as_array())
                .map(|arr| arr.iter().filter_map(|m| m.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let members: Vec<ScimMember> = member_ids.iter().map(|id| {
                ScimMember {
                    value: id.clone(),
                    reference: Some(format!("/Users/{}", id)),
                    display: None,
                    member_type: Some("User".to_string()),
                }
            }).collect();

            let scim_group = ScimGroup {
                schemas: vec!["urn:ietf:params:scim:schemas:core:2.0:Group".to_string()],
                id: Some(key.to_string()),
                external_id: None,
                display_name,
                members,
                meta: Some(ScimMeta {
                    resource_type: "Group".to_string(),
                    created: value.get("created_at").and_then(|v| v.as_str()).map(String::from),
                    last_modified: None,
                    location: Some(format!("/Groups/{}", key)),
                    version: None,
                }),
            };
            serde_json::to_value(scim_group).ok()
        })
        .collect();

    let total = groups.len() as u32;

    Ok(Json(ScimListResponse {
        schemas: vec!["urn:ietf:params:scim:api:messages:2.0:ListResponse".to_string()],
        total_results: total,
        start_index,
        items_per_page: count,
        resources: groups,
    }))
}
pub(super) async fn get_group(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<String>,
) -> Result<Json<ScimGroup>, (StatusCode, Json<ScimError>)> {
    info!("SCIM GET /Groups/{}", group_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ScimError {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:Error".to_string()],
            scim_type: None,
            detail: "No auth service".to_string(),
            status: 500,
        }))
    })?.lock().await;

    let metadata = auth_service.http_get(
        format!("{}/metadata/organization/{}", auth_service.api_url(), group_id),
    ).await.map_err(|e| {
        (StatusCode::NOT_FOUND, Json(ScimError {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:Error".to_string()],
            scim_type: Some("invalidValue".to_string()),
            detail: format!("Group not found: {}", e),
            status: 404,
        }))
    })?;

    let value_str = metadata.get("value").and_then(|v| v.as_str()).unwrap_or("{}");
    let value: serde_json::Value = serde_json::from_str(value_str).unwrap_or_default();

    let member_ids: Vec<String> = value.get("members")
        .and_then(|m| m.as_array())
        .map(|arr| arr.iter().filter_map(|m| m.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let members: Vec<ScimMember> = member_ids.iter().map(|id| {
        ScimMember {
            value: id.clone(),
            reference: Some(format!("/Users/{}", id)),
            display: None,
            member_type: Some("User".to_string()),
        }
    }).collect();

    Ok(Json(ScimGroup::from_metadata(&group_id, &value, members)))
}
pub(super) async fn create_group(
    State(state): State<Arc<AppState>>,
    Json(scim_group): Json<ScimGroup>,
) -> Result<(StatusCode, Json<ScimGroup>), (StatusCode, Json<ScimError>)> {
    info!("SCIM POST /Groups - creating: {}", scim_group.display_name);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ScimError {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:Error".to_string()],
            scim_type: None,
            detail: "No auth service".to_string(),
            status: 500,
        }))
    })?.lock().await;

    let member_ids: Vec<String> = scim_group.members.iter().map(|m| m.value.clone()).collect();
    let metadata_key = format!("group_{}", scim_group.id.as_deref().unwrap_or("new"));
    let metadata_value = serde_json::json!({
        "name": scim_group.display_name,
        "members": member_ids,
        "created_at": chrono::Utc::now().to_rfc3339()
    }).to_string();

    let body = serde_json::json!({
        "key": metadata_key,
        "value": metadata_value
    });

    auth_service.http_post(
        format!("{}/metadata/organization", auth_service.api_url()),
        body,
    ).await.map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(ScimError {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:Error".to_string()],
            scim_type: None,
            detail: format!("Failed to create group: {}", e),
            status: 400,
        }))
    })?;

    let mut created = scim_group;
    created.id = Some(metadata_key.clone());
    created.meta = Some(ScimMeta {
        resource_type: "Group".to_string(),
        created: Some(chrono::Utc::now().to_rfc3339()),
        last_modified: Some(chrono::Utc::now().to_rfc3339()),
        location: Some(format!("/Groups/{}", metadata_key)),
        version: None,
    });

    Ok((StatusCode::CREATED, Json(created)))
}
pub(super) async fn update_group(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<String>,
    Json(scim_group): Json<ScimGroup>,
) -> Result<Json<ScimGroup>, (StatusCode, Json<ScimError>)> {
    info!("SCIM PUT /Groups/{}", group_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ScimError {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:Error".to_string()],
            scim_type: None,
            detail: "No auth service".to_string(),
            status: 500,
        }))
    })?.lock().await;

    let member_ids: Vec<String> = scim_group.members.iter().map(|m| m.value.clone()).collect();
    let metadata_value = serde_json::json!({
        "name": scim_group.display_name,
        "members": member_ids,
        "updated_at": chrono::Utc::now().to_rfc3339()
    }).to_string();

    let body = serde_json::json!({
        "value": metadata_value
    });

    auth_service.http_put(
        format!("{}/metadata/organization/{}", auth_service.api_url(), group_id),
        body,
    ).await.map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(ScimError {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:Error".to_string()],
            scim_type: None,
            detail: format!("Failed to update group: {}", e),
            status: 400,
        }))
    })?;

    let mut updated = scim_group;
    updated.id = Some(group_id);
    Ok(Json(updated))
}
pub(super) async fn replace_group(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<String>,
    Json(scim_group): Json<ScimGroup>,
) -> Result<Json<ScimGroup>, (StatusCode, Json<ScimError>)> {
    update_group(State(state), Path(group_id), Json(scim_group)).await
}
pub(super) async fn delete_group(
    State(state): State<Arc<AppState>>,
    Path(group_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ScimError>)> {
    info!("SCIM DELETE /Groups/{}", group_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ScimError {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:Error".to_string()],
            scim_type: None,
            detail: "No auth service".to_string(),
            status: 500,
        }))
    })?.lock().await;

    auth_service.http_delete(
        format!("{}/metadata/organization/{}", auth_service.api_url(), group_id),
    ).await.map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(ScimError {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:Error".to_string()],
            scim_type: None,
            detail: format!("Failed to delete group: {}", e),
            status: 400,
        }))
    })?;

    Ok(StatusCode::NO_CONTENT)
}
