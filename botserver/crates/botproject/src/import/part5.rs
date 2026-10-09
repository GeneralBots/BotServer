use super::*;

impl ProjectImportService {
    pub(super) fn import_ms_project_mpp<R: Read>(
        &self,
        _reader: R,
        options: &ImportOptions,
    ) -> Result<ImportResult, String> {
        let project = Project {
            id: Uuid::new_v4(),
            organization_id: options.organization_id,
            name: "Imported MPP Project".to_string(),
            description: Some("MPP format requires conversion. Please export as XML from MS Project.".to_string()),
            start_date: Utc::now().date_naive(),
            end_date: None,
            status: ProjectStatus::Planning,
            owner_id: options.owner_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            settings: ProjectSettings::default(),
        };

        Ok(ImportResult {
            project,
            tasks: Vec::new(),
            resources: Vec::new(),
            assignments: Vec::new(),
            warnings: vec![ImportWarning {
                code: "MPP_NOT_SUPPORTED".to_string(),
                message: "Native MPP format is not fully supported. Please export as XML from MS Project for best results.".to_string(),
                source_element: None,
                suggested_action: Some("Use File > Save As > XML in MS Project".to_string()),
            }],
            errors: Vec::new(),
            stats: ImportStats {
                tasks_imported: 0,
                tasks_skipped: 0,
                resources_imported: 0,
                dependencies_imported: 0,
                assignments_imported: 0,
                custom_fields_imported: 0,
                import_duration_ms: 0,
            },
        })
    }

    pub(super) fn import_csv<R: Read>(
        &self,
        mut reader: R,
        options: &ImportOptions,
    ) -> Result<ImportResult, String> {
        let mut content = String::new();
        reader
            .read_to_string(&mut content)
            .map_err(|e| format!("Failed to read CSV content: {e}"))?;

        let mut tasks = Vec::new();
        let mut stats = ImportStats {
            tasks_imported: 0,
            tasks_skipped: 0,
            resources_imported: 0,
            dependencies_imported: 0,
            assignments_imported: 0,
            custom_fields_imported: 0,
            import_duration_ms: 0,
        };

        let project = Project {
            id: Uuid::new_v4(),
            organization_id: options.organization_id,
            name: "Imported CSV Project".to_string(),
            description: None,
            start_date: Utc::now().date_naive(),
            end_date: None,
            status: ProjectStatus::Planning,
            owner_id: options.owner_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            settings: ProjectSettings::default(),
        };

        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            return Ok(ImportResult {
                project,
                tasks: Vec::new(),
                resources: Vec::new(),
                assignments: Vec::new(),
                warnings: vec![ImportWarning {
                    code: "EMPTY_CSV".to_string(),
                    message: "CSV file is empty".to_string(),
                    source_element: None,
                    suggested_action: None,
                }],
                errors: Vec::new(),
                stats,
            });
        }

        let headers: Vec<&str> = lines[0].split(',').map(|s| s.trim()).collect();

        let name_idx = headers.iter().position(|h| h.eq_ignore_ascii_case("name") || h.eq_ignore_ascii_case("task"));
        let start_idx = headers.iter().position(|h| h.eq_ignore_ascii_case("start") || h.eq_ignore_ascii_case("start_date"));
        let end_idx = headers.iter().position(|h| h.eq_ignore_ascii_case("end") || h.eq_ignore_ascii_case("finish") || h.eq_ignore_ascii_case("end_date"));
        let duration_idx = headers.iter().position(|h| h.eq_ignore_ascii_case("duration"));

        for line in lines.iter().skip(1) {
            let fields: Vec<&str> = line.split(',').map(|s| s.trim()).collect();

            let name = name_idx
                .and_then(|i| fields.get(i))
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("Task {}", tasks.len() + 1));

            if name.is_empty() {
                stats.tasks_skipped += 1;
                continue;
            }

            let start_date = start_idx
                .and_then(|i| fields.get(i))
                .and_then(|s| parse_date_flexible(s))
                .unwrap_or_else(|| Utc::now().date_naive());

            let end_date = end_idx
                .and_then(|i| fields.get(i))
                .and_then(|s| parse_date_flexible(s))
                .unwrap_or(start_date);

            let duration_days = duration_idx
                .and_then(|i| fields.get(i))
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(1);

            let task = ProjectTask {
                id: Uuid::new_v4(),
                project_id: project.id,
                parent_id: None,
                name,
                description: None,
                task_type: TaskType::Task,
                start_date,
                end_date,
                duration_days,
                percent_complete: 0,
                status: TaskStatus::NotStarted,
                priority: TaskPriority::Normal,
                assigned_to: Vec::new(),
                dependencies: Vec::new(),
                estimated_hours: None,
                actual_hours: None,
                cost: None,
                notes: None,
                wbs: format!("{}", tasks.len() + 1),
                outline_level: 1,
                is_milestone: false,
                is_summary: false,
                is_critical: false,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            tasks.push(task);
            stats.tasks_imported += 1;
        }

        Ok(ImportResult {
            project,
            tasks,
            resources: Vec::new(),
            assignments: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
            stats,
        })
    }

    pub(super) fn import_json<R: Read>(
        &self,
        mut reader: R,
        options: &ImportOptions,
    ) -> Result<ImportResult, String> {
        let start = std::time::Instant::now();
        let mut content = String::new();
        reader
            .read_to_string(&mut content)
            .map_err(|e| format!("Failed to read JSON content: {e}"))?;

        #[derive(Deserialize)]
        struct JsonProject {
            name: String,
            description: Option<String>,
            start_date: Option<String>,
            tasks: Option<Vec<JsonTask>>,
        }

        #[derive(Deserialize)]
        struct JsonTask {
            name: String,
            start_date: Option<String>,
            end_date: Option<String>,
            duration: Option<u32>,
            progress: Option<u8>,
        }

        let json_project: JsonProject = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse JSON: {e}"))?;

        let project = Project {
            id: Uuid::new_v4(),
            organization_id: options.organization_id,
            name: json_project.name,
            description: json_project.description,
            start_date: json_project
                .start_date
                .as_ref()
                .and_then(|s| parse_date_flexible(s))
                .unwrap_or_else(|| Utc::now().date_naive()),
            end_date: None,
            status: ProjectStatus::Planning,
            owner_id: options.owner_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            settings: ProjectSettings::default(),
        };

        let mut tasks = Vec::new();
        let mut stats = ImportStats {
            tasks_imported: 0,
            tasks_skipped: 0,
            resources_imported: 0,
            dependencies_imported: 0,
            assignments_imported: 0,
            custom_fields_imported: 0,
            import_duration_ms: 0,
        };

        if let Some(json_tasks) = json_project.tasks {
            for (idx, json_task) in json_tasks.iter().enumerate() {
                let start_date = json_task
                    .start_date
                    .as_ref()
                    .and_then(|s| parse_date_flexible(s))
                    .unwrap_or_else(|| Utc::now().date_naive());

                let end_date = json_task
                    .end_date
                    .as_ref()
                    .and_then(|s| parse_date_flexible(s))
                    .unwrap_or(start_date);

                let task = ProjectTask {
                    id: Uuid::new_v4(),
                    project_id: project.id,
                    parent_id: None,
                    name: json_task.name.clone(),
                    description: None,
                    task_type: TaskType::Task,
                    start_date,
                    end_date,
                    duration_days: json_task.duration.unwrap_or(1),
                    percent_complete: json_task.progress.unwrap_or(0),
                    status: TaskStatus::NotStarted,
                    priority: TaskPriority::Normal,
                    assigned_to: Vec::new(),
                    dependencies: Vec::new(),
                    estimated_hours: None,
                    actual_hours: None,
                    cost: None,
                    notes: None,
                    wbs: format!("{}", idx + 1),
                    outline_level: 1,
                    is_milestone: false,
                    is_summary: false,
                    is_critical: false,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };

                tasks.push(task);
                stats.tasks_imported += 1;
            }
        }

        let duration = start.elapsed().as_millis() as u64;
        stats.import_duration_ms = duration;

        Ok(ImportResult {
            project,
            tasks,
            resources: Vec::new(),
            assignments: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
            stats,
        })
    }
}
