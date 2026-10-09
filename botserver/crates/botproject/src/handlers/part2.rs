use super::*;

pub async fn get_timeline(
    State(service): State<Arc<ProjectService>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<TimelineView>, ApiError> {
    require_project(&service, &user, project_id).await?;
    match service.get_timeline_view(project_id).await {
        Some(view) => Ok(Json(view)),
        None => Err(not_found("Project not found")),
    }
}
pub async fn get_critical_path(
    State(service): State<Arc<ProjectService>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<CriticalPathAnalysis>, ApiError> {
    require_project(&service, &user, project_id).await?;
    match service.calculate_critical_path_analysis(project_id).await {
        Some(analysis) => Ok(Json(analysis)),
        None => Err(not_found("Project not found or has no tasks")),
    }
}
pub async fn delete_task(
    State(service): State<Arc<ProjectService>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_task(&service, &user, task_id).await?;
    if service.delete_task(task_id).await {
        Ok(Json(serde_json::json!({ "success": true })))
    } else {
        Err(not_found("Task not found"))
    }
}
/// Applies a partial update to a task (issue #872). Recomputes `end_date` when
/// `start_date`/`duration_days` change and refreshes `updated_at`.
pub async fn update_task(
    State(service): State<Arc<ProjectService>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(task_id): Path<Uuid>,
    Json(req): Json<UpdateTaskRequest>,
) -> Result<Json<ProjectTask>, ApiError> {
    let mut task = require_task(&service, &user, task_id).await?;

    if let Some(name) = req.name {
        if !name.trim().is_empty() {
            task.name = name;
        }
    }
    if let Some(description) = req.description {
        task.description = description;
    }
    if let Some(task_type) = req.task_type {
        task.task_type = task_type;
    }
    if let Some(start_date) = req.start_date {
        task.start_date = start_date;
    }
    if let Some(duration_days) = req.duration_days {
        task.duration_days = duration_days;
        task.end_date = task.start_date + chrono::Duration::days(duration_days as i64);
    }
    if let Some(end_date) = req.end_date {
        task.end_date = end_date;
    }
    if let Some(status) = req.status {
        task.status = status;
    }
    if let Some(priority) = req.priority {
        task.priority = priority;
    }
    if let Some(assigned_to) = req.assigned_to {
        task.assigned_to = assigned_to;
    }
    if let Some(estimated_hours) = req.estimated_hours {
        task.estimated_hours = estimated_hours;
    }
    if let Some(actual_hours) = req.actual_hours {
        task.actual_hours = actual_hours;
    }
    if let Some(cost) = req.cost {
        task.cost = cost;
    }
    if let Some(notes) = req.notes {
        task.notes = notes;
    }
    if let Some(is_milestone) = req.is_milestone {
        task.is_milestone = is_milestone;
    }
    task.updated_at = Utc::now();

    service
        .update_task(task)
        .await
        .map(Json)
        .ok_or_else(|| not_found("Task not found"))
}
pub async fn remove_dependency(
    State(service): State<Arc<ProjectService>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(task_id): Path<Uuid>,
    Json(req): Json<RemoveDependencyRequest>,
) -> Result<Json<ProjectTask>, ApiError> {
    require_task(&service, &user, task_id).await?;
    service
        .remove_dependency(task_id, req.predecessor_id)
        .await
        .map(Json)
        .ok_or_else(|| not_found("Task not found"))
}
pub async fn create_resource(
    State(service): State<Arc<ProjectService>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateResourceRequest>,
) -> Result<Json<Resource>, ApiError> {
    require_project(&service, &user, project_id).await?;

    let resource = Resource {
        id: Uuid::new_v4(),
        project_id,
        user_id: req.user_id,
        name: req.name,
        resource_type: req.resource_type.unwrap_or(ResourceType::Work),
        email: req.email,
        max_units: req.max_units.unwrap_or(100.0),
        standard_rate: req.standard_rate,
        overtime_rate: req.overtime_rate,
        cost_per_use: req.cost_per_use,
        calendar_id: None,
        created_at: Utc::now(),
    };
    Ok(Json(service.create_resource(resource).await))
}
pub async fn list_resources(
    State(service): State<Arc<ProjectService>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<Resource>>, ApiError> {
    require_project(&service, &user, project_id).await?;
    Ok(Json(service.get_resources_for_project(project_id).await))
}
pub async fn delete_resource(
    State(service): State<Arc<ProjectService>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(resource_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_resource(&service, &user, resource_id).await?;
    if service.delete_resource(resource_id).await {
        Ok(Json(serde_json::json!({ "success": true })))
    } else {
        Err(not_found("Resource not found"))
    }
}
pub async fn assign_resource(
    State(service): State<Arc<ProjectService>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(task_id): Path<Uuid>,
    Json(req): Json<AssignResourceRequest>,
) -> Result<Json<ResourceAssignment>, ApiError> {
    require_task(&service, &user, task_id).await?;
    service
        .assign_resource(task_id, req.resource_id, req.units, req.work_hours)
        .await
        .map(Json)
        .ok_or_else(|| not_found("Task or resource not found"))
}
pub async fn list_assignments(
    State(service): State<Arc<ProjectService>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<Vec<ResourceAssignment>>, ApiError> {
    require_task(&service, &user, task_id).await?;
    Ok(Json(service.get_assignments_for_task(task_id).await))
}
/// Import a project from an uploaded file (MS Project XML/MPP, CSV, JSON).
/// Multipart fields: `file` (required) and optional `format` override.
pub async fn import_project(
    State(service): State<Arc<ProjectService>>,
    Extension(user): Extension<AuthenticatedUser>,
    mut multipart: Multipart,
) -> Result<Json<crate::import::ImportResult>, ApiError> {
    if user.user_id.is_nil() {
        return Err(forbidden());
    }

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut filename = "import.xml".to_string();
    let mut explicit_format: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name() {
            Some("file") => {
                if let Some(name) = field.file_name() {
                    filename = name.to_string();
                }
                match field.bytes().await {
                    Ok(bytes) => file_bytes = Some(bytes.to_vec()),
                    Err(e) => return Err(bad_request(&format!("Failed to read file: {e}"))),
                }
            }
            Some("format") => {
                if let Ok(text) = field.text().await {
                    explicit_format = Some(text);
                }
            }
            _ => {}
        }
    }

    let bytes = file_bytes.ok_or_else(|| bad_request("No file uploaded"))?;

    let format = match explicit_format.as_deref().map(str::to_lowercase).as_deref() {
        Some("xml") | Some("ms_project_xml") => ImportFormat::MsProjectXml,
        Some("mpp") | Some("ms_project_mpp") => ImportFormat::MsProjectMpp,
        Some("csv") => ImportFormat::Csv,
        Some("json") => ImportFormat::Json,
        _ => format_from_filename(&filename),
    };

    let options = ImportOptions {
        format,
        organization_id: user_org(&user),
        owner_id: user.user_id,
        ..ImportOptions::default()
    };

    let importer = ProjectImportService::new();
    let result = importer
        .import(&bytes[..], options)
        .map_err(|e| bad_request(&e))?;

    let result = service.import_data(result).await;
    Ok(Json(result))
}
