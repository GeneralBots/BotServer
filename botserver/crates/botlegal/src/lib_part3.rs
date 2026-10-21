use super::*;

pub async fn handle_get_consent(
    State(pool): State<Arc<DbPool>>,
    Path(consent_id): Path<Uuid>,
) -> Result<Json<Option<CookieConsent>>, LegalError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| LegalError::Database(e.to_string()))?;

        let db_consent: Option<DbCookieConsent> = cookie_consents::table
            .find(consent_id)
            .first(&mut conn)
            .optional()
            .map_err(|e| LegalError::Database(e.to_string()))?;

        Ok::<_, LegalError>(db_consent.map(db_consent_to_consent))
    })
    .await
    .map_err(|e| LegalError::Internal(e.to_string()))??;

    Ok(Json(result))
}
#[derive(Debug, Deserialize)]
pub struct ConsentBySessionQuery {
    pub session_id: String,
}
pub async fn handle_get_consent_by_session(
    State(pool): State<Arc<DbPool>>,
    Query(query): Query<ConsentBySessionQuery>,
) -> Result<Json<Option<CookieConsent>>, LegalError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| LegalError::Database(e.to_string()))?;

        let db_consent: Option<DbCookieConsent> = cookie_consents::table
            .filter(cookie_consents::session_id.eq(&query.session_id))
            .order(cookie_consents::consent_given_at.desc())
            .first(&mut conn)
            .optional()
            .map_err(|e| LegalError::Database(e.to_string()))?;

        Ok::<_, LegalError>(db_consent.map(db_consent_to_consent))
    })
    .await
    .map_err(|e| LegalError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub async fn handle_update_consent(
    State(pool): State<Arc<DbPool>>,
    Path(consent_id): Path<Uuid>,
    Json(req): Json<CookieConsentRequest>,
) -> Result<Json<CookieConsent>, LegalError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| LegalError::Database(e.to_string()))?;
        let now = Utc::now();

        let mut db_consent: DbCookieConsent = cookie_consents::table
            .find(consent_id)
            .first(&mut conn)
            .map_err(|_| LegalError::NotFound("Consent not found".to_string()))?;

        let previous_consents = serde_json::json!({
            "necessary": db_consent.consent_necessary,
            "analytics": db_consent.consent_analytics,
            "marketing": db_consent.consent_marketing,
            "preferences": db_consent.consent_preferences,
            "functional": db_consent.consent_functional,
        });

        let mut consents = req.consents;
        consents.insert(CookieCategory::Necessary, true);

        db_consent.consent_necessary = true;
        db_consent.consent_analytics = *consents.get(&CookieCategory::Analytics).unwrap_or(&false);
        db_consent.consent_marketing = *consents.get(&CookieCategory::Marketing).unwrap_or(&false);
        db_consent.consent_preferences = *consents.get(&CookieCategory::Preferences).unwrap_or(&false);
        db_consent.consent_functional = *consents.get(&CookieCategory::Functional).unwrap_or(&false);
        db_consent.consent_updated_at = now;

        let new_consents = serde_json::json!({
            "necessary": db_consent.consent_necessary,
            "analytics": db_consent.consent_analytics,
            "marketing": db_consent.consent_marketing,
            "preferences": db_consent.consent_preferences,
            "functional": db_consent.consent_functional,
        });

        diesel::update(cookie_consents::table.find(consent_id))
            .set(&db_consent)
            .execute(&mut conn)
            .map_err(|e| LegalError::Database(e.to_string()))?;

        let history = DbConsentHistory {
            id: Uuid::new_v4(),
            consent_id,
            action: "update".to_string(),
            previous_consents,
            new_consents,
            ip_address: None,
            user_agent: req.user_agent,
            created_at: now,
        };

        diesel::insert_into(consent_history::table)
            .values(&history)
            .execute(&mut conn)
            .map_err(|e| LegalError::Database(e.to_string()))?;

        Ok::<_, LegalError>(db_consent_to_consent(db_consent))
    })
    .await
    .map_err(|e| LegalError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub async fn handle_get_cookie_policy(
    State(_pool): State<Arc<DbPool>>,
) -> Result<Json<CookiePolicy>, LegalError> {
    Ok(Json(get_default_cookie_policy()))
}
pub async fn handle_list_documents(
    State(pool): State<Arc<DbPool>>,
    Query(query): Query<ListDocumentsQuery>,
) -> Result<Json<Vec<LegalDocument>>, LegalError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| LegalError::Database(e.to_string()))?;
        let branch_id = get_branch_id(&mut conn);

        let mut db_query = legal_documents::table
            .filter(legal_documents::branch_id.eq(branch_id))
            .into_boxed();

        if let Some(doc_type) = query.document_type {
            db_query = db_query.filter(legal_documents::document_type.eq(doc_type));
        }

        if query.active_only.unwrap_or(true) {
            db_query = db_query.filter(legal_documents::is_active.eq(true));
        }

        let db_docs: Vec<DbLegalDocument> = db_query
            .order(legal_documents::created_at.desc())
            .load(&mut conn)
            .map_err(|e| LegalError::Database(e.to_string()))?;

        let docs: Vec<LegalDocument> = db_docs.into_iter().map(db_document_to_document).collect();
        Ok::<_, LegalError>(docs)
    })
    .await
    .map_err(|e| LegalError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub async fn handle_get_document(
    State(pool): State<Arc<DbPool>>,
    Path(slug): Path<String>,
) -> Result<Json<Option<LegalDocument>>, LegalError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| LegalError::Database(e.to_string()))?;
        let branch_id = get_branch_id(&mut conn);

        let db_doc: Option<DbLegalDocument> = legal_documents::table
            .filter(legal_documents::branch_id.eq(branch_id))
            .filter(legal_documents::slug.eq(&slug))
            .filter(legal_documents::is_active.eq(true))
            .first(&mut conn)
            .optional()
            .map_err(|e| LegalError::Database(e.to_string()))?;

        Ok::<_, LegalError>(db_doc.map(db_document_to_document))
    })
    .await
    .map_err(|e| LegalError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub async fn handle_create_document(
    State(pool): State<Arc<DbPool>>,
    Json(req): Json<CreateDocumentRequest>,
) -> Result<Json<LegalDocument>, LegalError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| LegalError::Database(e.to_string()))?;
        let branch_id = get_branch_id(&mut conn);
        let now = Utc::now();

        let (org_id, bot_id) = org_and_bot(&mut conn, branch_id);
        let db_doc = DbLegalDocument {
            id: Uuid::new_v4(),
            org_id,
            bot_id,
            slug: req.slug,
            title: req.title,
            content: req.content,
            document_type: req.document_type.to_string(),
            version: req.version.unwrap_or_else(|| "1.0.0".to_string()),
            effective_date: now,
            is_active: true,
            requires_acceptance: req.requires_acceptance.unwrap_or(false),
            metadata: serde_json::json!({}),
            created_by: None,
            created_at: now,
            updated_at: now,
            branch_id,
        };

        diesel::insert_into(legal_documents::table)
            .values(&db_doc)
            .execute(&mut conn)
            .map_err(|e| LegalError::Database(e.to_string()))?;

        Ok::<_, LegalError>(db_document_to_document(db_doc))
    })
    .await
    .map_err(|e| LegalError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub async fn handle_update_document(
    State(pool): State<Arc<DbPool>>,
    Path(slug): Path<String>,
    Json(req): Json<UpdateDocumentRequest>,
) -> Result<Json<LegalDocument>, LegalError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| LegalError::Database(e.to_string()))?;
        let branch_id = get_branch_id(&mut conn);
        let now = Utc::now();

        let mut db_doc: DbLegalDocument = legal_documents::table
            .filter(legal_documents::branch_id.eq(branch_id))
            .filter(legal_documents::slug.eq(&slug))
            .first(&mut conn)
            .map_err(|_| LegalError::NotFound("Document not found".to_string()))?;

        if let Some(title) = req.title {
            db_doc.title = title;
        }
        if let Some(content) = req.content {
            let version_record = DbDocumentVersion {
                id: Uuid::new_v4(),
                document_id: db_doc.id,
                version: db_doc.version.clone(),
                content: db_doc.content.clone(),
                change_summary: None,
                created_by: None,
                created_at: now,
            };

            diesel::insert_into(legal_document_versions::table)
                .values(&version_record)
                .execute(&mut conn)
                .map_err(|e| LegalError::Database(e.to_string()))?;

            db_doc.content = content;
        }
        if let Some(is_active) = req.is_active {
            db_doc.is_active = is_active;
        }
        if let Some(requires_acceptance) = req.requires_acceptance {
            db_doc.requires_acceptance = requires_acceptance;
        }
        db_doc.updated_at = now;

        diesel::update(legal_documents::table.find(db_doc.id))
            .set(&db_doc)
            .execute(&mut conn)
            .map_err(|e| LegalError::Database(e.to_string()))?;

        Ok::<_, LegalError>(db_document_to_document(db_doc))
    })
    .await
    .map_err(|e| LegalError::Internal(e.to_string()))??;

    Ok(Json(result))
}
