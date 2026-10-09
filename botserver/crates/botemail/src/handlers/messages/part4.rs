use super::*;

pub async fn create_label(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(request): Json<CreateLabelRequest>,
) -> Result<Json<ApiResponse<()>>, EmailError> {
    let user_id = extract_user_from_session(&headers)
        .map_err(|_| EmailError("Authentication required".to_string()))?;
    let pool = state.pool.clone();

    tokio::task::spawn_blocking(move || {
        let mut db_conn = pool.get().map_err(|e| format!("DB connection error: {e}"))?;
        let label_id = Uuid::new_v4();
        let bot_id = Uuid::nil();

        diesel::sql_query(
            "INSERT INTO email_labels (id, user_id, bot_id, name, color, is_system, created_at)
             VALUES ($1, $2, $3, $4, $5, false, NOW())"
        )
        .bind::<diesel::sql_types::Uuid, _>(label_id)
        .bind::<diesel::sql_types::Uuid, _>(user_id)
        .bind::<diesel::sql_types::Uuid, _>(bot_id)
        .bind::<diesel::sql_types::Text, _>(&request.name)
        .bind::<diesel::sql_types::Text, _>(&request.color)
        .execute(&mut db_conn)
        .map_err(|e| format!("Failed to create label: {e}"))?;

        Ok::<_, String>(())
    })
    .await
    .map_err(|e| EmailError(format!("Task join error: {e}")))?
    .map_err(EmailError)?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(()),
        message: Some("Label created successfully".to_string()),
    }))
}
