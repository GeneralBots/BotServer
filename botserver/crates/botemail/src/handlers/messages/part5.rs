use super::*;

pub async fn add_label_to_emails_bulk(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(request): Json<BulkAddLabelRequest>,
) -> Result<Json<ApiResponse<()>>, EmailError> {
    let user_id = extract_user_from_session(&headers)
        .map_err(|_| EmailError("Authentication required".to_string()))?;
    let pool = state.pool.clone();

    tokio::task::spawn_blocking(move || {
        use crate::models::EmailLabelRow;
        let mut db_conn = pool.get().map_err(|e| format!("DB connection error: {e}"))?;

        let label_row: Option<EmailLabelRow> = diesel::sql_query(
            "SELECT id, name, color FROM email_labels WHERE user_id = $1 AND name = $2 LIMIT 1"
        )
        .bind::<diesel::sql_types::Uuid, _>(user_id)
        .bind::<diesel::sql_types::Text, _>(&request.label_name)
        .get_result(&mut db_conn)
        .optional()
        .map_err(|e| format!("Failed to find label: {e}"))?;

        let label_id = match label_row {
            Some(row) => row.id,
            None => {
                let new_label_id = Uuid::new_v4();
                diesel::sql_query(
                    "INSERT INTO email_labels (id, user_id, bot_id, name, color, is_system, created_at)
                     VALUES ($1, $2, $3, $4, '#3b82f6', false, NOW())"
                )
                .bind::<diesel::sql_types::Uuid, _>(new_label_id)
                .bind::<diesel::sql_types::Uuid, _>(user_id)
                .bind::<diesel::sql_types::Uuid, _>(Uuid::nil())
                .bind::<diesel::sql_types::Text, _>(&request.label_name)
                .execute(&mut db_conn)
                .map_err(|e| format!("Failed to auto-create label: {e}"))?;
                new_label_id
            }
        };

        for msg_id in &request.ids {
            let assignment_id = Uuid::new_v4();
            diesel::sql_query(
                "INSERT INTO email_label_assignments (id, email_message_id, label_id, assigned_at)
                 VALUES ($1, $2, $3, NOW())
                 ON CONFLICT DO NOTHING"
            )
            .bind::<diesel::sql_types::Uuid, _>(assignment_id)
            .bind::<diesel::sql_types::Varchar, _>(&msg_id.to_string())
            .bind::<diesel::sql_types::Uuid, _>(label_id)
            .execute(&mut db_conn)
            .map_err(|e| format!("Failed to assign label to message: {}", e))?;
        }

        Ok::<_, String>(())
    })
    .await
    .map_err(|e| EmailError(format!("Task join error: {e}")))?
    .map_err(EmailError)?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(()),
        message: Some("Labels assigned successfully".to_string()),
    }))
}
