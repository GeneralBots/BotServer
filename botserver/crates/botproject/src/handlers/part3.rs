use super::*;

/// Export a project to MS Project XML (default), CSV, or JSON as a file
/// download.
pub async fn export_project(
    State(service): State<Arc<ProjectService>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(project_id): Path<Uuid>,
    Query(query): Query<ExportQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let project = require_project(&service, &user, project_id).await?;
    let tasks = service.get_tasks_for_project(project_id).await;
    let resources = service.get_resources_for_project(project_id).await;
    let assignments = service.get_assignments_for_project(project_id).await;

    let format = query.format.unwrap_or_else(|| "xml".to_string()).to_lowercase();

    let (content_type, body) = match format.as_str() {
        "xml" | "mpp" => (
            "application/xml; charset=utf-8",
            export::export_ms_project_xml(&project, &tasks, &resources, &assignments),
        ),
        "csv" => ("text/csv; charset=utf-8", export::export_csv(&project, &tasks)),
        "json" => (
            "application/json; charset=utf-8",
            export::export_json(&project, &tasks),
        ),
        _ => return Err(bad_request("Unsupported export format (use xml, csv, or json)")),
    };

    let filename = format!(
        "{}.{}",
        sanitize_filename(&project.name),
        if format.as_str() == "mpp" { "xml" } else { &format }
    );

    Ok((
        [
            (axum::http::header::CONTENT_TYPE, content_type.to_string()),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        body,
    ))
}
pub(super) fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "project".to_string()
    } else {
        cleaned
    }
}
