use super::*;

pub(super) fn set_mfa_flag(
    state: &Arc<AppState>,
    user_id: &str,
    enabled: bool,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
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
    let value = if enabled { "true" } else { "false" };
    diesel::sql_query(
        "INSERT INTO user_preferences (id, user_id, preference_key, preference_value, created_at, updated_at) \
         VALUES ($1, $2, 'mfa_enabled', $3::jsonb, NOW(), NOW()) \
         ON CONFLICT (user_id, preference_key) \
         DO UPDATE SET preference_value = EXCLUDED.preference_value, updated_at = NOW()",
    )
    .bind::<diesel::sql_types::Uuid, _>(uuid::Uuid::new_v4())
    .bind::<diesel::sql_types::Uuid, _>(user_uuid)
    .bind::<diesel::sql_types::Text, _>(value)
    .execute(&mut conn)
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to update 2FA: {e}"),
                details: None,
            }),
        )
    })?;
    Ok(())
}
pub async fn enable_2fa(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
    Json(_req): Json<Enable2faRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    info!("Enabling 2FA for user: {}", user_id);

    set_mfa_flag(&state, &user_id, true)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "user_id": user_id,
        "message": "2FA enabled successfully"
    })))
}
pub async fn disable_2fa(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    info!("Disabling 2FA for user: {}", user_id);

    set_mfa_flag(&state, &user_id, false)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "user_id": user_id,
        "message": "2FA disabled successfully"
    })))
}
pub async fn get_user_devices(
    State(_state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    info!("Getting devices for user: {}", user_id);

    Ok(Json(serde_json::json!({
        "user_id": user_id,
        "devices": []
    })))
}
pub async fn get_user_sessions(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    info!("Getting sessions for user: {}", user_id);

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
    struct SessionRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: uuid::Uuid,
        #[diesel(sql_type = diesel::sql_types::Varchar)]
        title: String,
        #[diesel(sql_type = diesel::sql_types::Int4)]
        message_count: i32,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)]
        last_activity: DateTime<Utc>,
    }

    let sessions: Vec<SessionRow> = diesel::sql_query(
        "SELECT id, title, message_count, last_activity FROM user_sessions WHERE user_id = $1 ORDER BY last_activity DESC LIMIT 50",
    )
    .bind::<diesel::sql_types::Uuid, _>(user_uuid)
    .load(&mut conn)
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to load sessions: {e}"),
                details: None,
            }),
        )
    })?;

    let sessions_json: Vec<serde_json::Value> = sessions
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "title": s.title,
                "message_count": s.message_count,
                "last_activity": s.last_activity.to_rfc3339(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "user_id": user_id,
        "sessions": sessions_json
    })))
}
