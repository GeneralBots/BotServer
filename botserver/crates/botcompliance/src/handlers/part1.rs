use super::*;

pub async fn handle_list_checks(
    State(pool): State<Arc<crate::DbPool>>,
    Query(query): Query<ListChecksQuery>,
) -> Result<Json<Vec<ComplianceCheckResult>>, ComplianceError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);
        let branch_id = BRANCH_ID_PLACEHOLDER;

        let mut db_query = compliance_checks::table
            .filter(compliance_checks::branch_id.eq(branch_id))
            .into_boxed();

        if let Some(check_type) = query.check_type {
            db_query = db_query.filter(compliance_checks::check_type.eq(check_type));
        }

        if let Some(status) = query.status {
            db_query = db_query.filter(compliance_checks::status.eq(Some(status)));
        }

        let db_checks: Vec<DbComplianceCheck> = db_query
            .order(compliance_checks::updated_at.desc())
            .offset(offset)
            .limit(limit)
            .load(&mut conn)
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        let mut results = Vec::new();
        for check in db_checks {
            results.push(db_check_to_result(check, vec![]));
        }

        Ok::<_, ComplianceError>(results)
    })
    .await
    .map_err(|e| ComplianceError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub async fn handle_run_check(
    State(pool): State<Arc<crate::DbPool>>,
    Json(req): Json<RunCheckRequest>,
) -> Result<Json<Vec<ComplianceCheckResult>>, ComplianceError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| ComplianceError::Database(e.to_string()))?;
        let now = Utc::now();
        let branch_id = BRANCH_ID_PLACEHOLDER;

        let controls = match req.framework {
            ComplianceFramework::Gdpr => vec![
                ("gdpr_7.2", "Data Retention Policy", 95.0),
                ("gdpr_5.1.f", "Data Protection Measures", 100.0),
                ("gdpr_6.1", "Lawful Basis for Processing", 98.0),
            ],
            ComplianceFramework::Soc2 => vec![("cc6.1", "Logical and Physical Access Controls", 94.0)],
            ComplianceFramework::Iso27001 => vec![("a.8.1", "Inventory of Assets", 90.0)],
            ComplianceFramework::Hipaa => vec![("164.312", "Technical Safeguards", 85.0)],
            ComplianceFramework::PciDss => vec![("req_3", "Protect Stored Cardholder Data", 88.0)],
        };

        let mut results = Vec::new();
        for (control_id, control_name, score) in controls {
            let db_check = DbComplianceCheck {
                id: Uuid::new_v4(),
                org_id: None,
                bot_id: None,
                branch_id,
                check_type: req.framework.to_string(),
                status: Some("compliant".to_string()),
                target_type: Some("control".to_string()),
                target_id: None,
                result: Some(serde_json::json!({
                    "control_id": control_id,
                    "control_name": control_name,
                    "score": score,
                })),
                checked_at: Some(now),
                checked_by: None,
                created_at: now,
                updated_at: now,
            };

            diesel::insert_into(compliance_checks::table)
                .values(&db_check)
                .execute(&mut conn)
                .map_err(|e| ComplianceError::Database(e.to_string()))?;

            results.push(db_check_to_result(db_check, vec![]));
        }

        Ok::<_, ComplianceError>(results)
    })
    .await
    .map_err(|e| ComplianceError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub async fn handle_get_check(
    State(pool): State<Arc<crate::DbPool>>,
    Path(check_id): Path<Uuid>,
) -> Result<Json<Option<ComplianceCheckResult>>, ComplianceError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        let db_check: Option<DbComplianceCheck> = compliance_checks::table
            .find(check_id)
            .first(&mut conn)
            .optional()
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        match db_check {
            Some(check) => {
                Ok::<_, ComplianceError>(Some(db_check_to_result(check, vec![])))
            }
            None => Ok(None),
        }
    })
    .await
    .map_err(|e| ComplianceError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub async fn handle_list_issues(
    State(pool): State<Arc<crate::DbPool>>,
    Query(query): Query<ListIssuesQuery>,
) -> Result<Json<Vec<ComplianceIssueResult>>, ComplianceError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        let limit = query.limit.unwrap_or(50);
        let offset = query.offset.unwrap_or(0);
        let branch_id = BRANCH_ID_PLACEHOLDER;

        let mut db_query = compliance_issues::table
            .filter(compliance_issues::branch_id.eq(branch_id))
            .into_boxed();

        if let Some(severity) = query.severity {
            db_query = db_query.filter(compliance_issues::severity.eq(severity));
        }

        if let Some(status) = query.status {
            db_query = db_query.filter(compliance_issues::status.eq(status));
        }

        if let Some(assigned_to) = query.assigned_to {
            db_query = db_query.filter(compliance_issues::assigned_to.eq(assigned_to));
        }

        let db_issues: Vec<DbComplianceIssue> = db_query
            .order(compliance_issues::created_at.desc())
            .offset(offset)
            .limit(limit)
            .load(&mut conn)
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        let issues: Vec<ComplianceIssueResult> =
            db_issues.into_iter().map(db_issue_to_result).collect();
        Ok::<_, ComplianceError>(issues)
    })
    .await
    .map_err(|e| ComplianceError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub async fn handle_create_issue(
    State(pool): State<Arc<crate::DbPool>>,
    Json(req): Json<CreateIssueRequest>,
) -> Result<Json<ComplianceIssueResult>, ComplianceError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| ComplianceError::Database(e.to_string()))?;
        let now = Utc::now();
        let branch_id = BRANCH_ID_PLACEHOLDER;

        let db_issue = DbComplianceIssue {
            id: Uuid::new_v4(),
            org_id: Uuid::nil(),
            bot_id: Uuid::nil(),
            check_id: req.check_id,
            branch_id,
            title: req.title,
            severity: req.severity.to_string(),
            status: "open".to_string(),
            description: req.description,
            assigned_to: req.assigned_to,
            remediation: req.remediation,
            due_date: req.due_date,
            resolved_at: None,
            resolved_by: None,
            resolution_notes: None,
            created_at: now,
            updated_at: now,
        };

        diesel::insert_into(compliance_issues::table)
            .values(&db_issue)
            .execute(&mut conn)
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        Ok::<_, ComplianceError>(db_issue_to_result(db_issue))
    })
    .await
    .map_err(|e| ComplianceError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub async fn handle_update_issue(
    State(pool): State<Arc<crate::DbPool>>,
    Path(issue_id): Path<Uuid>,
    Json(req): Json<UpdateIssueRequest>,
) -> Result<Json<ComplianceIssueResult>, ComplianceError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool
            .get()
            .map_err(|e| ComplianceError::Database(e.to_string()))?;
        let now = Utc::now();

        let mut db_issue: DbComplianceIssue = compliance_issues::table
            .find(issue_id)
            .first(&mut conn)
            .map_err(|_| ComplianceError::NotFound("Issue not found".to_string()))?;

        if let Some(severity) = req.severity {
            db_issue.severity = severity.to_string();
        }
        if let Some(title) = req.title {
            db_issue.title = title;
        }
        if let Some(description) = req.description {
            db_issue.description = description;
        }
        if let Some(remediation) = req.remediation {
            db_issue.remediation = Some(remediation);
        }
        if let Some(due_date) = req.due_date {
            db_issue.due_date = Some(due_date);
        }
        if let Some(assigned_to) = req.assigned_to {
            db_issue.assigned_to = Some(assigned_to);
        }
        if let Some(status) = req.status {
            if status == "resolved" {
                db_issue.resolved_at = Some(now);
            }
            db_issue.status = status;
        }
        db_issue.updated_at = now;

        diesel::update(compliance_issues::table.find(issue_id))
            .set(&db_issue)
            .execute(&mut conn)
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        Ok::<_, ComplianceError>(db_issue_to_result(db_issue))
    })
    .await
    .map_err(|e| ComplianceError::Internal(e.to_string()))??;

    Ok(Json(result))
}
