use super::*;

/// `GET /api/compliance/frameworks/:framework_id` — framework + controls + coverage
pub async fn handle_get_framework(
    State(pool): State<Arc<crate::DbPool>>,
    Path(framework_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ComplianceError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| ComplianceError::Database(e.to_string()))?;

        let fw: DbComplianceFramework = compliance_frameworks::table
            .find(framework_id)
            .first(&mut conn)
            .map_err(|_| ComplianceError::NotFound("Framework not found".to_string()))?;

        let controls: Vec<DbComplianceControl> = compliance_controls::table
            .filter(compliance_controls::framework_id.eq(framework_id))
            .order(compliance_controls::control_id.asc())
            .load(&mut conn)
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        let control_ids: Vec<Uuid> = controls.iter().map(|c| c.id).collect();

        let evidence_rows: Vec<DbComplianceControlEvidence> = if control_ids.is_empty() {
            Vec::new()
        } else {
            compliance_control_evidence::table
                .filter(compliance_control_evidence::control_id.eq_any(&control_ids))
                .load(&mut conn)
                .map_err(|e| ComplianceError::Database(e.to_string()))?
        };

        let mut evidence_by_control: std::collections::HashMap<Uuid, Vec<serde_json::Value>> =
            std::collections::HashMap::new();
        for ev in &evidence_rows {
            evidence_by_control
                .entry(ev.control_id)
                .or_default()
                .push(evidence_json(ev));
        }

        let mut total = 0usize;
        let mut covered = 0usize;
        let mut controls_json = Vec::new();

        for c in &controls {
            let evidence = evidence_by_control.get(&c.id).cloned().unwrap_or_default();
            let approved = evidence
                .iter()
                .filter(|e| e["status"] == serde_json::json!("approved"))
                .count();
            let has_evidence = approved > 0;
            total += 1;
            if has_evidence {
                covered += 1;
            }
            let mut cj = control_json(c);
            cj["evidence"] = serde_json::json!(evidence);
            cj["has_evidence"] = serde_json::json!(has_evidence);
            controls_json.push(cj);
        }

        let coverage = if total > 0 {
            (covered as f64 / total as f64 * 100.0 * 100.0).round() / 100.0
        } else {
            0.0
        };

        let mut out = framework_json(&fw);
        out["controls"] = serde_json::json!(controls_json);
        out["total_controls"] = serde_json::json!(total);
        out["controls_with_evidence"] = serde_json::json!(covered);
        out["coverage_percent"] = serde_json::json!(coverage);
        Ok::<_, ComplianceError>(out)
    })
    .await
    .map_err(|e| ComplianceError::Internal(e.to_string()))??;

    Ok(Json(result))
}
/// `POST /api/compliance/controls`
pub async fn handle_create_control(
    State(pool): State<Arc<crate::DbPool>>,
    Json(req): Json<CreateControlRequest>,
) -> Result<Json<serde_json::Value>, ComplianceError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| ComplianceError::Database(e.to_string()))?;
        let now = Utc::now();
        let branch_id = BRANCH_ID_PLACEHOLDER;

        let control_id = req.control_id.trim().to_string();
        if control_id.is_empty() || req.title.trim().is_empty() {
            return Err(ComplianceError::Validation(
                "Control id and title are required".to_string(),
            ));
        }

        let existing: i64 = compliance_controls::table
            .filter(compliance_controls::framework_id.eq(req.framework_id))
            .filter(compliance_controls::control_id.eq(&control_id))
            .count()
            .get_result(&mut conn)
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        if existing > 0 {
            return Err(ComplianceError::Validation(format!(
                "Control {control_id} already exists in this framework"
            )));
        }

        let control = DbComplianceControl {
            id: Uuid::new_v4(),
            branch_id,
            framework_id: req.framework_id,
            control_id,
            title: req.title.trim().to_string(),
            description: req.description,
            category: req.category,
            is_mandatory: req.is_mandatory.unwrap_or(true),
            version: "1.0.0".to_string(),
            status: "active".to_string(),
            created_by: req.created_by,
            created_at: now,
            updated_at: now,
        };

        diesel::insert_into(compliance_controls::table)
            .values(&control)
            .execute(&mut conn)
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        audit(
            &mut conn,
            branch_id,
            control.created_by,
            "control.create",
            "compliance_control",
            Some(control.id),
            Some(serde_json::json!({ "framework_id": control.framework_id, "control_id": control.control_id })),
        );

        Ok::<_, ComplianceError>(control_json(&control))
    })
    .await
    .map_err(|e| ComplianceError::Internal(e.to_string()))??;

    Ok(Json(result))
}
/// `PUT /api/compliance/controls/:control_id`
pub async fn handle_update_control(
    State(pool): State<Arc<crate::DbPool>>,
    Path(control_id): Path<Uuid>,
    Json(req): Json<UpdateControlRequest>,
) -> Result<Json<serde_json::Value>, ComplianceError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| ComplianceError::Database(e.to_string()))?;
        let now = Utc::now();

        let mut control: DbComplianceControl = compliance_controls::table
            .find(control_id)
            .first(&mut conn)
            .map_err(|_| ComplianceError::NotFound("Control not found".to_string()))?;

        if let Some(title) = req.title {
            control.title = title;
        }
        if let Some(description) = req.description {
            control.description = Some(description);
        }
        if let Some(category) = req.category {
            control.category = Some(category);
        }
        if let Some(is_mandatory) = req.is_mandatory {
            control.is_mandatory = is_mandatory;
        }
        if let Some(status) = req.status {
            control.status = status;
        }
        control.updated_at = now;

        diesel::update(compliance_controls::table.find(control_id))
            .set(&control)
            .execute(&mut conn)
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        audit(
            &mut conn,
            control.branch_id,
            None,
            "control.update",
            "compliance_control",
            Some(control.id),
            None,
        );

        Ok::<_, ComplianceError>(control_json(&control))
    })
    .await
    .map_err(|e| ComplianceError::Internal(e.to_string()))??;

    Ok(Json(result))
}
/// `POST /api/compliance/evidence/attach`
pub async fn handle_attach_evidence(
    State(pool): State<Arc<crate::DbPool>>,
    Json(req): Json<AttachEvidenceRequest>,
) -> Result<Json<serde_json::Value>, ComplianceError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| ComplianceError::Database(e.to_string()))?;
        let now = Utc::now();
        let branch_id = BRANCH_ID_PLACEHOLDER;

        // Control must exist and belong to the branch scope.
        let _control: DbComplianceControl = compliance_controls::table
            .find(req.control_id)
            .first(&mut conn)
            .map_err(|_| ComplianceError::NotFound("Control not found".to_string()))?;

        let ev = DbComplianceControlEvidence {
            id: Uuid::new_v4(),
            branch_id,
            control_id: req.control_id,
            file_path: req.file_path,
            description: req.description,
            evidence_type: req.evidence_type.unwrap_or_else(|| "artifact".to_string()),
            status: "draft".to_string(),
            owner_id: req.owner_id,
            approved_by: None,
            approved_at: None,
            created_at: now,
            updated_at: now,
        };

        diesel::insert_into(compliance_control_evidence::table)
            .values(&ev)
            .execute(&mut conn)
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        audit(
            &mut conn,
            branch_id,
            ev.owner_id,
            "evidence.attach",
            "compliance_control_evidence",
            Some(ev.id),
            Some(serde_json::json!({ "control_id": ev.control_id, "file_path": ev.file_path })),
        );

        Ok::<_, ComplianceError>(evidence_json(&ev))
    })
    .await
    .map_err(|e| ComplianceError::Internal(e.to_string()))??;

    Ok(Json(result))
}
/// `POST /api/compliance/evidence/:evidence_id/approve`
pub async fn handle_approve_evidence(
    State(pool): State<Arc<crate::DbPool>>,
    Path(evidence_id): Path<Uuid>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ComplianceError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| ComplianceError::Database(e.to_string()))?;
        let now = Utc::now();

        let mut ev: DbComplianceControlEvidence = compliance_control_evidence::table
            .find(evidence_id)
            .first(&mut conn)
            .map_err(|_| ComplianceError::NotFound("Evidence not found".to_string()))?;

        let approver: Option<Uuid> = body
            .get("approved_by")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok());

        ev.status = "approved".to_string();
        ev.approved_by = approver;
        ev.approved_at = Some(now);
        ev.updated_at = now;

        diesel::update(compliance_control_evidence::table.find(evidence_id))
            .set(&ev)
            .execute(&mut conn)
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        audit(
            &mut conn,
            ev.branch_id,
            approver,
            "evidence.approve",
            "compliance_control_evidence",
            Some(ev.id),
            None,
        );

        Ok::<_, ComplianceError>(evidence_json(&ev))
    })
    .await
    .map_err(|e| ComplianceError::Internal(e.to_string()))??;

    Ok(Json(result))
}
