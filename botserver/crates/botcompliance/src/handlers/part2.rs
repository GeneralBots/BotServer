use super::*;

pub async fn handle_list_audit_logs(
    State(pool): State<Arc<crate::DbPool>>,
    Query(query): Query<ListAuditLogsQuery>,
) -> Result<Json<Vec<AuditLogEntry>>, ComplianceError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        let limit = query.limit.unwrap_or(100);
        let offset = query.offset.unwrap_or(0);
        let branch_id = BRANCH_ID_PLACEHOLDER;

        let mut db_query = compliance_audit_log::table
            .filter(compliance_audit_log::branch_id.eq(branch_id))
            .into_boxed();

        if let Some(action) = query.action {
            db_query = db_query.filter(compliance_audit_log::action.eq(action));
        }

        if let Some(actor_id) = query.actor_id {
            db_query = db_query.filter(compliance_audit_log::user_id.eq(actor_id));
        }

        if let Some(target_type) = query.target_type {
            db_query = db_query.filter(compliance_audit_log::resource_type.eq(target_type));
        }

        if let Some(from_date) = query.from_date {
            db_query = db_query.filter(compliance_audit_log::created_at.ge(from_date));
        }

        if let Some(to_date) = query.to_date {
            db_query = db_query.filter(compliance_audit_log::created_at.le(to_date));
        }

        let db_logs: Vec<DbAuditLog> = db_query
            .order(compliance_audit_log::created_at.desc())
            .offset(offset)
            .limit(limit)
            .load(&mut conn)
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        let logs: Vec<AuditLogEntry> = db_logs.into_iter().map(db_audit_to_entry).collect();
        Ok::<_, ComplianceError>(logs)
    })
    .await
    .map_err(|e| ComplianceError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub async fn handle_create_audit_log(
    State(pool): State<Arc<crate::DbPool>>,
    Json(req): Json<CreateAuditLogRequest>,
) -> Result<Json<AuditLogEntry>, ComplianceError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| ComplianceError::Database(e.to_string()))?;
        let now = Utc::now();
        let branch_id = BRANCH_ID_PLACEHOLDER;

        let metadata = serde_json::to_value(&req.metadata.unwrap_or_default()).unwrap_or(serde_json::json!({}));

        let db_log = DbAuditLog {
            id: Uuid::new_v4(),
            org_id: Uuid::nil(),
            bot_id: Uuid::nil(),
            branch_id,
            event_type: req.event_type.to_string(),
            user_id: req.actor_id.or(req.user_id),
            resource_type: req.target_type,
            resource_id: req.resource_id,
            action: req.action,
            result: req.result.to_string(),
            ip_address: req.ip_address,
            user_agent: req.user_agent,
            metadata,
            created_at: now,
        };

        diesel::insert_into(compliance_audit_log::table)
            .values(&db_log)
            .execute(&mut conn)
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        Ok::<_, ComplianceError>(db_audit_to_entry(db_log))
    })
    .await
    .map_err(|e| ComplianceError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub async fn handle_create_training(
    State(pool): State<Arc<crate::DbPool>>,
    Json(req): Json<CreateTrainingRequest>,
) -> Result<Json<TrainingRecord>, ComplianceError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| ComplianceError::Database(e.to_string()))?;
        let now = Utc::now();
        let branch_id = BRANCH_ID_PLACEHOLDER;

        let db_training = DbTrainingRecord {
            id: Uuid::new_v4(),
            org_id: Uuid::nil(),
            bot_id: Uuid::nil(),
            branch_id,
            user_id: req.user_id,
            training_type: req.training_type.to_string(),
            training_name: req.training_name.clone(),
            provider: req.provider.clone(),
            score: req.score,
            passed: req.passed,
            completion_date: now,
            valid_until: req.valid_until,
            certificate_url: req.certificate_url.clone(),
            metadata: serde_json::json!({}),
            created_at: now,
        };

        diesel::insert_into(compliance_training_records::table)
            .values(&db_training)
            .execute(&mut conn)
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        Ok::<_, ComplianceError>(TrainingRecord {
            id: db_training.id,
            user_id: req.user_id,
            training_type: req.training_type,
            training_name: req.training_name,
            provider: req.provider,
            score: req.score,
            passed: req.passed,
            completion_date: now,
            valid_until: req.valid_until,
            certificate_url: req.certificate_url,
        })
    })
    .await
    .map_err(|e| ComplianceError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub async fn handle_list_training(
    State(pool): State<Arc<crate::DbPool>>,
) -> Result<Json<Vec<TrainingRecord>>, ComplianceError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| ComplianceError::Database(e.to_string()))?;
        let branch_id = BRANCH_ID_PLACEHOLDER;

        let db_records: Vec<DbTrainingRecord> = compliance_training_records::table
            .filter(compliance_training_records::branch_id.eq(branch_id))
            .order(compliance_training_records::completion_date.desc())
            .load(&mut conn)
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        let records: Vec<TrainingRecord> = db_records
            .into_iter()
            .map(|r| TrainingRecord {
                id: r.id,
                user_id: r.user_id,
                training_type: r.training_type.parse().unwrap_or(TrainingType::SecurityAwareness),
                training_name: r.training_name,
                provider: r.provider,
                score: r.score,
                passed: r.passed,
                completion_date: r.completion_date,
                valid_until: r.valid_until,
                certificate_url: r.certificate_url,
            })
            .collect();

        Ok::<_, ComplianceError>(records)
    })
    .await
    .map_err(|e| ComplianceError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub async fn handle_list_risks(
    State(pool): State<Arc<crate::DbPool>>,
) -> Result<Json<Vec<serde_json::Value>>, ComplianceError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| ComplianceError::Database(e.to_string()))?;
        let branch_id = BRANCH_ID_PLACEHOLDER;

        let items: Vec<DbRiskAssessment> = compliance_risk_assessments::table
            .filter(compliance_risk_assessments::branch_id.eq(branch_id))
            .order(compliance_risk_assessments::created_at.desc())
            .load(&mut conn)
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        let json_items: Vec<serde_json::Value> =
            items.into_iter().map(db_risk_assessment_to_json).collect();

        Ok::<_, ComplianceError>(json_items)
    })
    .await
    .map_err(|e| ComplianceError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub async fn handle_run_check_by_id(
    State(pool): State<Arc<crate::DbPool>>,
    Path(check_id): Path<Uuid>,
) -> Result<Json<ComplianceCheckResult>, ComplianceError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| ComplianceError::Database(e.to_string()))?;
        let now = Utc::now();

        let mut db_check: DbComplianceCheck = compliance_checks::table
            .find(check_id)
            .first(&mut conn)
            .map_err(|_| ComplianceError::NotFound("Check not found".to_string()))?;

        db_check.checked_at = Some(now);
        db_check.status = Some("in_progress".to_string());

        diesel::update(compliance_checks::table.find(check_id))
            .set(&db_check)
            .execute(&mut conn)
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        Ok::<_, ComplianceError>(db_check_to_result(db_check, vec![]))
    })
    .await
    .map_err(|e| ComplianceError::Internal(e.to_string()))??;

    Ok(Json(result))
}
