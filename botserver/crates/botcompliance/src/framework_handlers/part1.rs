use super::*;

pub(super) fn audit(
    conn: &mut diesel::PgConnection,
    branch_id: Uuid,
    actor_id: Option<Uuid>,
    action: &str,
    target_type: &str,
    target_id: Option<Uuid>,
    details: Option<serde_json::Value>,
) {
    let now = Utc::now();
    let result = diesel::insert_into(compliance_audit_log::table)
        .values((
            compliance_audit_log::id.eq(Uuid::new_v4()),
            compliance_audit_log::org_id.eq(Uuid::nil()),
            compliance_audit_log::bot_id.eq(Uuid::nil()),
            compliance_audit_log::branch_id.eq(branch_id),
            compliance_audit_log::event_type.eq(action),
            compliance_audit_log::user_id.eq(actor_id),
            compliance_audit_log::resource_type.eq(target_type),
            compliance_audit_log::resource_id.eq(target_id.map(|id| id.to_string()).unwrap_or_default()),
            compliance_audit_log::action.eq(action),
            compliance_audit_log::result.eq("success"),
            compliance_audit_log::ip_address.eq(Option::<String>::None),
            compliance_audit_log::user_agent.eq(Option::<String>::None),
            compliance_audit_log::metadata.eq(details.unwrap_or_else(|| serde_json::json!({}))),
            compliance_audit_log::created_at.eq(now),
        ))
        .execute(conn);
    if let Err(e) = result {
        log::warn!("compliance audit write failed: {e}");
    }
}
pub(super) fn framework_json(fw: &DbComplianceFramework) -> serde_json::Value {
    serde_json::json!({
        "id": fw.id,
        "name": fw.name,
        "version": fw.version,
        "description": fw.description,
        "framework_key": fw.framework_key,
        "status": fw.status,
        "created_by": fw.created_by,
        "created_at": fw.created_at,
        "updated_at": fw.updated_at,
    })
}
pub(super) fn control_json(c: &DbComplianceControl) -> serde_json::Value {
    serde_json::json!({
        "id": c.id,
        "framework_id": c.framework_id,
        "control_id": c.control_id,
        "title": c.title,
        "description": c.description,
        "category": c.category,
        "is_mandatory": c.is_mandatory,
        "version": c.version,
        "status": c.status,
        "created_at": c.created_at,
    })
}
pub(super) fn evidence_json(e: &DbComplianceControlEvidence) -> serde_json::Value {
    serde_json::json!({
        "id": e.id,
        "control_id": e.control_id,
        "file_path": e.file_path,
        "description": e.description,
        "evidence_type": e.evidence_type,
        "status": e.status,
        "owner_id": e.owner_id,
        "approved_by": e.approved_by,
        "approved_at": e.approved_at,
        "created_at": e.created_at,
    })
}
/// `GET /api/compliance/frameworks`
pub async fn handle_list_frameworks(
    State(pool): State<Arc<crate::DbPool>>,
    Query(query): Query<ListFrameworksQuery>,
) -> Result<Json<Vec<serde_json::Value>>, ComplianceError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| ComplianceError::Database(e.to_string()))?;
        let branch_id = BRANCH_ID_PLACEHOLDER;

        let mut db_query = compliance_frameworks::table
            .filter(compliance_frameworks::branch_id.eq(branch_id))
            .into_boxed();

        if let Some(status) = query.status {
            db_query = db_query.filter(compliance_frameworks::status.eq(status));
        }

        let frameworks: Vec<DbComplianceFramework> = db_query
            .order(compliance_frameworks::name.asc())
            .limit(query.limit.unwrap_or(100))
            .load(&mut conn)
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        let mut items = Vec::new();
        for fw in &frameworks {
            let controls_count: i64 = compliance_controls::table
                .filter(compliance_controls::framework_id.eq(fw.id))
                .count()
                .get_result(&mut conn)
                .unwrap_or(0);
            let mut item = framework_json(fw);
            item["controls_count"] = serde_json::json!(controls_count);
            items.push(item);
        }
        Ok::<_, ComplianceError>(items)
    })
    .await
    .map_err(|e| ComplianceError::Internal(e.to_string()))??;

    Ok(Json(result))
}
/// `POST /api/compliance/frameworks`
pub async fn handle_create_framework(
    State(pool): State<Arc<crate::DbPool>>,
    Json(req): Json<CreateFrameworkRequest>,
) -> Result<Json<serde_json::Value>, ComplianceError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| ComplianceError::Database(e.to_string()))?;
        let now = Utc::now();
        let branch_id = BRANCH_ID_PLACEHOLDER;

        let name = req.name.trim().to_string();
        if name.is_empty() {
            return Err(ComplianceError::Validation("Framework name is required".to_string()));
        }
        let framework_key = req
            .framework_key
            .unwrap_or_else(|| name.to_lowercase().replace(' ', "_"));

        let fw = DbComplianceFramework {
            id: Uuid::new_v4(),
            branch_id,
            name,
            version: req.version.unwrap_or_else(|| "1.0.0".to_string()),
            description: req.description,
            framework_key,
            status: "active".to_string(),
            created_by: req.created_by,
            created_at: now,
            updated_at: now,
        };

        diesel::insert_into(compliance_frameworks::table)
            .values(&fw)
            .execute(&mut conn)
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        audit(
            &mut conn,
            branch_id,
            fw.created_by,
            "framework.create",
            "compliance_framework",
            Some(fw.id),
            Some(serde_json::json!({ "name": fw.name, "version": fw.version })),
        );

        Ok::<_, ComplianceError>(framework_json(&fw))
    })
    .await
    .map_err(|e| ComplianceError::Internal(e.to_string()))??;

    Ok(Json(result))
}
/// `PUT /api/compliance/frameworks/:framework_id`
pub async fn handle_update_framework(
    State(pool): State<Arc<crate::DbPool>>,
    Path(framework_id): Path<Uuid>,
    Json(req): Json<UpdateFrameworkRequest>,
) -> Result<Json<serde_json::Value>, ComplianceError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| ComplianceError::Database(e.to_string()))?;
        let now = Utc::now();

        let mut fw: DbComplianceFramework = compliance_frameworks::table
            .find(framework_id)
            .first(&mut conn)
            .map_err(|_| ComplianceError::NotFound("Framework not found".to_string()))?;

        if let Some(name) = req.name {
            fw.name = name;
        }
        if let Some(version) = req.version {
            fw.version = version;
        }
        if let Some(description) = req.description {
            fw.description = Some(description);
        }
        if let Some(status) = req.status {
            if !["active", "archived"].contains(&status.as_str()) {
                return Err(ComplianceError::Validation("Status must be active or archived".to_string()));
            }
            fw.status = status;
        }
        fw.updated_at = now;

        diesel::update(compliance_frameworks::table.find(framework_id))
            .set(&fw)
            .execute(&mut conn)
            .map_err(|e| ComplianceError::Database(e.to_string()))?;

        audit(
            &mut conn,
            fw.branch_id,
            None,
            "framework.update",
            "compliance_framework",
            Some(fw.id),
            Some(serde_json::json!({ "name": fw.name, "version": fw.version, "status": fw.status })),
        );

        Ok::<_, ComplianceError>(framework_json(&fw))
    })
    .await
    .map_err(|e| ComplianceError::Internal(e.to_string()))??;

    Ok(Json(result))
}
/// `POST /api/compliance/frameworks/:framework_id/archive`
pub async fn handle_archive_framework(
    State(pool): State<Arc<crate::DbPool>>,
    Path(framework_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ComplianceError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| ComplianceError::Database(e.to_string()))?;
        let now = Utc::now();

        diesel::update(compliance_frameworks::table.find(framework_id))
            .set((
                compliance_frameworks::status.eq("archived"),
                compliance_frameworks::updated_at.eq(now),
            ))
            .execute(&mut conn)
            .map_err(|_| ComplianceError::NotFound("Framework not found".to_string()))?;

        audit(
            &mut conn,
            BRANCH_ID_PLACEHOLDER,
            None,
            "framework.archive",
            "compliance_framework",
            Some(framework_id),
            None,
        );

        Ok::<_, ComplianceError>(serde_json::json!({ "ok": true, "id": framework_id }))
    })
    .await
    .map_err(|e| ComplianceError::Internal(e.to_string()))??;

    Ok(Json(result))
}
