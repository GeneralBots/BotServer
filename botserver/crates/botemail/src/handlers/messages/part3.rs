use super::*;

pub async fn archive_emails_bulk(
    State(state): State<Arc<AppState>>,
    Json(request): Json<BulkActionRequest>,
) -> Result<Json<ApiResponse<()>>, EmailError> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut db_conn = pool.get().map_err(|e| format!("DB connection error: {e}"))?;
        diesel::sql_query("UPDATE email_messages SET folder = 'archive' WHERE id = ANY($1)")
            .bind::<diesel::sql_types::Array<diesel::sql_types::Uuid>, _>(&request.ids)
            .execute(&mut db_conn)
            .map_err(|e| format!("Failed to archive emails: {e}"))?;
        Ok::<_, String>(())
    })
    .await
    .map_err(|e| EmailError(format!("Task join error: {e}")))?
    .map_err(EmailError)?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(()),
        message: Some("Emails archived successfully".to_string()),
    }))
}
pub async fn mark_emails_read_bulk(
    State(state): State<Arc<AppState>>,
    Json(request): Json<BulkMarkReadRequest>,
) -> Result<Json<ApiResponse<()>>, EmailError> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut db_conn = pool.get().map_err(|e| format!("DB connection error: {e}"))?;
        diesel::sql_query("UPDATE email_messages SET is_read = $1 WHERE id = ANY($2)")
            .bind::<diesel::sql_types::Bool, _>(request.is_read)
            .bind::<diesel::sql_types::Array<diesel::sql_types::Uuid>, _>(&request.ids)
            .execute(&mut db_conn)
            .map_err(|e| format!("Failed to mark emails as read: {e}"))?;
        Ok::<_, String>(())
    })
    .await
    .map_err(|e| EmailError(format!("Task join error: {e}")))?
    .map_err(EmailError)?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(()),
        message: Some("Emails updated successfully".to_string()),
    }))
}
pub async fn delete_emails_bulk(
    State(state): State<Arc<AppState>>,
    Json(request): Json<BulkActionRequest>,
) -> Result<Json<ApiResponse<()>>, EmailError> {
    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut db_conn = pool.get().map_err(|e| format!("DB connection error: {e}"))?;
        diesel::sql_query("UPDATE email_messages SET folder = 'trash' WHERE id = ANY($1)")
            .bind::<diesel::sql_types::Array<diesel::sql_types::Uuid>, _>(&request.ids)
            .execute(&mut db_conn)
            .map_err(|e| format!("Failed to delete emails: {e}"))?;
        Ok::<_, String>(())
    })
    .await
    .map_err(|e| EmailError(format!("Task join error: {e}")))?
    .map_err(EmailError)?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(()),
        message: Some("Emails deleted successfully".to_string()),
    }))
}
