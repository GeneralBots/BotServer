use super::*;

pub(super) fn forbidden() -> ApiError {
    (
        StatusCode::FORBIDDEN,
        Json(serde_json::json!({ "error": "Forbidden" })),
    )
}
pub(super) fn not_found(what: &str) -> ApiError {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": what })),
    )
}
pub(super) fn bad_request(what: &str) -> ApiError {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({ "error": what })),
    )
}
/// Detect the import format from a file extension, defaulting to MS Project
/// XML (the interchange format MS Project writes via File > Save As > XML).
pub(super) fn format_from_filename(filename: &str) -> ImportFormat {
    let lower = filename.to_lowercase();
    if lower.ends_with(".mpp") {
        ImportFormat::MsProjectMpp
    } else if lower.ends_with(".csv") {
        ImportFormat::Csv
    } else if lower.ends_with(".json") {
        ImportFormat::Json
    } else {
        ImportFormat::MsProjectXml
    }
}
/// Effective tenant scope for a user. When the auth middleware has not
/// attached an organization (service accounts, some legacy sessions) we fall
/// back to the user's own id so a user never sees another user's data.
pub(super) fn user_org(user: &AuthenticatedUser) -> Uuid {
    user.organization_id.unwrap_or(user.user_id)
}
/// Projects are tenant-scoped by `organization_id`. Admins can read across
/// tenants; everyone else is confined to their own organization.
pub(super) fn can_access_project(user: &AuthenticatedUser, project: &Project) -> bool {
    user.is_admin() || project.organization_id == user_org(user)
}
/// Load a project and verify the caller may access it, or return 403/404.
pub(super) async fn require_project(
    service: &ProjectService,
    user: &AuthenticatedUser,
    project_id: Uuid,
) -> Result<Project, ApiError> {
    let project = service
        .get_project(project_id)
        .await
        .ok_or_else(|| not_found("Project not found"))?;
    if !can_access_project(user, &project) {
        return Err(forbidden());
    }
    Ok(project)
}
/// Load a task and verify the caller may access its owning project.
pub(super) async fn require_task(
    service: &ProjectService,
    user: &AuthenticatedUser,
    task_id: Uuid,
) -> Result<ProjectTask, ApiError> {
    let task = service
        .get_task(task_id)
        .await
        .ok_or_else(|| not_found("Task not found"))?;
    let project = service
        .get_project(task.project_id)
        .await
        .ok_or_else(|| not_found("Project not found"))?;
    if !can_access_project(user, &project) {
        return Err(forbidden());
    }
    Ok(task)
}
/// Load a resource and verify the caller may access its owning project.
pub(super) async fn require_resource(
    service: &ProjectService,
    user: &AuthenticatedUser,
    resource_id: Uuid,
) -> Result<Resource, ApiError> {
    let resource = service
        .get_resource(resource_id)
        .await
        .ok_or_else(|| not_found("Resource not found"))?;
    let project = service
        .get_project(resource.project_id)
        .await
        .ok_or_else(|| not_found("Project not found"))?;
    if !can_access_project(user, &project) {
        return Err(forbidden());
    }
    Ok(resource)
}
pub async fn create_project(
    State(service): State<Arc<ProjectService>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<CreateProjectRequest>,
) -> Result<Json<Project>, ApiError> {
    if user.user_id.is_nil() {
        return Err(forbidden());
    }

    let project = Project {
        id: Uuid::new_v4(),
        organization_id: user_org(&user),
        name: req.name,
        description: req.description,
        start_date: req.start_date,
        end_date: req.end_date,
        status: ProjectStatus::Planning,
        owner_id: user.user_id,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        settings: ProjectSettings::default(),
    };

    let created = service.create_project(project).await;
    Ok(Json(created))
}
pub async fn list_projects(
    State(service): State<Arc<ProjectService>>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Json<Vec<Project>> {
    if user.is_admin() {
        Json(service.get_all_projects().await)
    } else {
        Json(service.get_projects_for_organization(user_org(&user)).await)
    }
}
pub async fn get_project(
    State(service): State<Arc<ProjectService>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Project>, ApiError> {
    let project = require_project(&service, &user, project_id).await?;
    Ok(Json(project))
}
pub async fn delete_project(
    State(service): State<Arc<ProjectService>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_project(&service, &user, project_id).await?;
    if service.delete_project(project_id).await {
        Ok(Json(serde_json::json!({ "success": true })))
    } else {
        Err(not_found("Project not found"))
    }
}
/// Applies a partial update to a project (issue #873). Loads the project,
/// overlays the provided fields, refreshes `updated_at`, and persists.
pub async fn update_project(
    State(service): State<Arc<ProjectService>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(project_id): Path<Uuid>,
    Json(req): Json<UpdateProjectRequest>,
) -> Result<Json<Project>, ApiError> {
    let mut project = require_project(&service, &user, project_id).await?;

    if let Some(name) = req.name {
        if !name.trim().is_empty() {
            project.name = name;
        }
    }
    if let Some(description) = req.description {
        project.description = description;
    }
    if let Some(start_date) = req.start_date {
        project.start_date = start_date;
    }
    if let Some(end_date) = req.end_date {
        project.end_date = end_date;
    }
    if let Some(status) = req.status {
        project.status = status;
    }
    project.updated_at = Utc::now();

    service
        .update_project(project)
        .await
        .map(Json)
        .ok_or_else(|| not_found("Project not found"))
}
/// Single-purpose lifecycle transition endpoint (issue #873).
pub async fn update_project_status(
    State(service): State<Arc<ProjectService>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(project_id): Path<Uuid>,
    Json(req): Json<UpdateProjectStatusRequest>,
) -> Result<Json<Project>, ApiError> {
    let mut project = require_project(&service, &user, project_id).await?;

    project.status = req.status;
    project.updated_at = Utc::now();

    service
        .update_project(project)
        .await
        .map(Json)
        .ok_or_else(|| not_found("Project not found"))
}
pub async fn create_task(
    State(service): State<Arc<ProjectService>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(project_id): Path<Uuid>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<Json<ProjectTask>, ApiError> {
    require_project(&service, &user, project_id).await?;

    let end_date = req.start_date + chrono::Duration::days(req.duration_days as i64);

    let task = ProjectTask {
        id: Uuid::new_v4(),
        project_id,
        parent_id: req.parent_id,
        name: req.name,
        description: req.description,
        task_type: req.task_type.unwrap_or(TaskType::Task),
        start_date: req.start_date,
        end_date,
        duration_days: req.duration_days,
        percent_complete: 0,
        status: TaskStatus::NotStarted,
        priority: req.priority.unwrap_or(TaskPriority::Normal),
        assigned_to: Vec::new(),
        dependencies: Vec::new(),
        estimated_hours: None,
        actual_hours: None,
        cost: None,
        notes: None,
        wbs: "1".to_string(),
        outline_level: 1,
        is_milestone: req.is_milestone.unwrap_or(false),
        is_summary: false,
        is_critical: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let created = service.create_task(task).await;
    Ok(Json(created))
}
pub async fn get_tasks(
    State(service): State<Arc<ProjectService>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<Vec<ProjectTask>>, ApiError> {
    require_project(&service, &user, project_id).await?;
    let tasks = service.get_tasks_for_project(project_id).await;
    Ok(Json(tasks))
}
pub async fn update_task_progress(
    State(service): State<Arc<ProjectService>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(task_id): Path<Uuid>,
    Json(req): Json<UpdateProgressRequest>,
) -> Result<Json<ProjectTask>, ApiError> {
    require_task(&service, &user, task_id).await?;
    match service.update_task_progress(task_id, req.percent_complete).await {
        Some(task) => Ok(Json(task)),
        None => Err(not_found("Task not found")),
    }
}
pub async fn add_dependency(
    State(service): State<Arc<ProjectService>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(task_id): Path<Uuid>,
    Json(req): Json<AddDependencyRequest>,
) -> Result<Json<ProjectTask>, ApiError> {
    require_task(&service, &user, task_id).await?;
    match service
        .add_dependency(task_id, req.predecessor_id, req.dependency_type, req.lag_days.unwrap_or(0))
        .await
    {
        Some(task) => Ok(Json(task)),
        None => Err(not_found("Task not found")),
    }
}
pub async fn get_gantt_chart(
    State(service): State<Arc<ProjectService>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<GanttChartData>, ApiError> {
    require_project(&service, &user, project_id).await?;
    match service.get_gantt_chart_data(project_id).await {
        Some(data) => Ok(Json(data)),
        None => Err(not_found("Project not found")),
    }
}
