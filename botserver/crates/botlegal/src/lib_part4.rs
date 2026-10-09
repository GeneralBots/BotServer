use super::*;

pub async fn handle_request_data_deletion(
    State(pool): State<Arc<DbPool>>,
    Path(user_id): Path<Uuid>,
    Json(req): Json<DataDeletionRequest>,
) -> Result<Json<DataDeletionResult>, LegalError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| LegalError::Database(e.to_string()))?;
        let branch_id = get_branch_id(&mut conn);
        let now = Utc::now();
        let token = Uuid::new_v4().to_string();

        let db_request = DbDeletionRequest {
            id: Uuid::new_v4(),
            branch_id,
            user_id,
            request_type: "full".to_string(),
            status: "pending".to_string(),
            reason: req.reason,
            requested_at: now,
            scheduled_for: Some(now + chrono::Duration::days(30)),
            completed_at: None,
            confirmation_token: token.clone(),
            confirmed_at: None,
            processed_by: None,
            notes: None,
            created_at: now,
            updated_at: now,
        };

        diesel::insert_into(data_deletion_requests::table)
            .values(&db_request)
            .execute(&mut conn)
            .map_err(|e| LegalError::Database(e.to_string()))?;

        let deleted_consents = diesel::delete(
            cookie_consents::table
                .filter(cookie_consents::branch_id.eq(branch_id))
                .filter(cookie_consents::user_id.eq(user_id)),
        )
        .execute(&mut conn)
        .unwrap_or(0);

        Ok::<_, LegalError>(DataDeletionResult {
            user_id,
            consents_deleted: deleted_consents as i32,
            deleted_at: now,
            confirmation_token: token,
        })
    })
    .await
    .map_err(|e| LegalError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub async fn handle_export_user_data(
    State(pool): State<Arc<DbPool>>,
    Path(user_id): Path<Uuid>,
    Json(req): Json<DataExportRequest>,
) -> Result<Json<UserDataExport>, LegalError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| LegalError::Database(e.to_string()))?;
        let branch_id = get_branch_id(&mut conn);
        let now = Utc::now();

        let format = req.format.unwrap_or_else(|| "json".to_string());
        let sections = req.sections.unwrap_or_else(|| vec!["all".to_string()]);

        let db_export = DbExportRequest {
            id: Uuid::new_v4(),
            branch_id,
            user_id,
            status: "completed".to_string(),
            format: format.clone(),
            include_sections: serde_json::to_value(&sections).unwrap_or_default(),
            requested_at: now,
            started_at: Some(now),
            completed_at: Some(now),
            file_url: None,
            file_size: None,
            expires_at: Some(now + chrono::Duration::days(7)),
            error_message: None,
            created_at: now,
        };

        diesel::insert_into(data_export_requests::table)
            .values(&db_export)
            .execute(&mut conn)
            .map_err(|e| LegalError::Database(e.to_string()))?;

        let db_consents: Vec<DbCookieConsent> = cookie_consents::table
            .filter(cookie_consents::branch_id.eq(branch_id))
            .filter(cookie_consents::user_id.eq(user_id))
            .load(&mut conn)
            .unwrap_or_default();

        let consents: Vec<CookieConsent> = db_consents.into_iter().map(db_consent_to_consent).collect();

        Ok::<_, LegalError>(UserDataExport {
            user_id,
            exported_at: now,
            consents,
            format,
        })
    })
    .await
    .map_err(|e| LegalError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub struct LegalService {}
impl LegalService {
    pub fn new() -> Self {
        Self {}
    }
}
impl Default for LegalService {
    fn default() -> Self {
        Self::new()
    }
}
pub fn configure_legal_routes() -> Router<Arc<DbPool>> {
    Router::new()
        .route("/api/legal/consent", post(handle_record_consent))
        .route("/api/legal/consent/:consent_id", get(handle_get_consent).put(handle_update_consent))
        .route("/api/legal/consent/session", get(handle_get_consent_by_session))
        .route("/api/legal/cookies/policy", get(handle_get_cookie_policy))
        .route("/api/legal/documents", get(handle_list_documents).post(handle_create_document))
        .route("/api/legal/documents/:slug", get(handle_get_document).put(handle_update_document))
        .route("/api/legal/gdpr/delete/:user_id", post(handle_request_data_deletion))
        .route("/api/legal/gdpr/export/:user_id", post(handle_export_user_data))
}
pub(super) fn get_branch_id(conn: &mut diesel::PgConnection) -> Uuid {
    use diesel::dsl::exists;
    use diesel::select;
    let _ = select(exists(branches::table.find(Uuid::nil())))
        .get_result::<bool>(conn);
    Uuid::nil()
}
