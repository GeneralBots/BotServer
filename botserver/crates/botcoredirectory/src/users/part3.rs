use super::*;

pub async fn get_user_permissions(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Result<Json<UserPermissionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Getting permissions for user: {}", user_id);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

    let roles = match auth_service.get_user_memberships(&user_id, 0, 100).await {
        Ok(data) => {
            data.get("result")
                .and_then(|r| r.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|m| {
                            m.get("roles")
                                .and_then(|r| r.as_array())
                                .map(|roles| {
                                    roles.iter()
                                        .filter_map(|r| r.as_str().map(String::from))
                                        .collect::<Vec<String>>()
                                })
                        })
                        .flatten()
                        .collect::<Vec<String>>()
                })
                .unwrap_or_default()
        }
        Err(_) => vec![],
    };

    Ok(Json(UserPermissionsResponse {
        user_id: user_id.clone(),
        permissions: vec![],
        roles,
    }))
}
pub async fn get_user_presence(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    info!("Getting presence for user: {}", user_id);

    let user_uuid = user_id.parse::<uuid::Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid user id".to_string(),
                details: None,
            }),
        )
    })?;
    let mut conn = state.conn.get().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("DB error: {e}"),
                details: None,
            }),
        )
    })?;

    #[derive(diesel::QueryableByName)]
    struct LastActivityRow {
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
        last_activity: Option<DateTime<Utc>>,
    }

    let row: Option<LastActivityRow> = diesel::sql_query(
        "SELECT last_activity FROM user_sessions WHERE user_id = $1 ORDER BY last_activity DESC LIMIT 1",
    )
    .bind::<diesel::sql_types::Uuid, _>(user_uuid)
    .get_result(&mut conn)
    .ok();

    let last_seen = row.as_ref().and_then(|r| r.last_activity);
    let online = last_seen
        .map(|t| Utc::now().signed_duration_since(t) < chrono::Duration::minutes(5))
        .unwrap_or(false);

    Ok(Json(serde_json::json!({
        "user_id": user_id,
        "status": if online { "online" } else { "offline" },
        "last_seen": last_seen.map(|t| t.to_rfc3339()),
        "online": online
    })))
}
pub async fn get_user_activity(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    info!("Getting activity for user: {}", user_id);

    let user_uuid = user_id.parse::<uuid::Uuid>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid user id".to_string(),
                details: None,
            }),
        )
    })?;
    let mut conn = state.conn.get().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("DB error: {e}"),
                details: None,
            }),
        )
    })?;

    #[derive(diesel::QueryableByName)]
    struct ActivityRow {
        #[diesel(sql_type = diesel::sql_types::Varchar)]
        title: String,
        #[diesel(sql_type = diesel::sql_types::Int4)]
        message_count: i32,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)]
        updated_at: DateTime<Utc>,
    }

    let rows: Vec<ActivityRow> = diesel::sql_query(
        "SELECT title, message_count, updated_at FROM user_sessions WHERE user_id = $1 ORDER BY updated_at DESC LIMIT 10",
    )
    .bind::<diesel::sql_types::Uuid, _>(user_uuid)
    .load(&mut conn)
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to load activity: {e}"),
                details: None,
            }),
        )
    })?;

    let recent_activity: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "type": "session",
                "title": r.title,
                "message_count": r.message_count,
                "timestamp": r.updated_at.to_rfc3339(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "user_id": user_id,
        "recent_activity": recent_activity,
        "total_sessions": rows.len()
    })))
}
