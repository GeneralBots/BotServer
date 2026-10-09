use super::*;

impl ProjectImportService {
    pub(super) fn import_ms_project_xml<R: Read>(
        &self,
        mut reader: R,
        options: &ImportOptions,
    ) -> Result<ImportResult, String> {
        let mut xml_content = String::new();
        reader
            .read_to_string(&mut xml_content)
            .map_err(|e| format!("Failed to read XML content: {e}"))?;

        let ms_project: MsProjectXml = quick_xml::de::from_str(&xml_content)
            .map_err(|e| format!("Failed to parse MS Project XML: {e}"))?;

        let mut warnings = Vec::new();
        let errors = Vec::new();
        let mut stats = ImportStats {
            tasks_imported: 0,
            tasks_skipped: 0,
            resources_imported: 0,
            dependencies_imported: 0,
            assignments_imported: 0,
            custom_fields_imported: 0,
            import_duration_ms: 0,
        };

        let project_name = ms_project
            .title
            .or(ms_project.name)
            .unwrap_or_else(|| "Imported Project".to_string());

        let start_date = ms_project
            .start_date
            .as_ref()
            .and_then(|s| parse_ms_date(s))
            .unwrap_or_else(|| Utc::now().date_naive());

        let end_date = ms_project
            .finish_date
            .as_ref()
            .and_then(|s| parse_ms_date(s));

        let project = Project {
            id: Uuid::new_v4(),
            organization_id: options.organization_id,
            name: project_name,
            description: None,
            start_date,
            end_date,
            status: ProjectStatus::Planning,
            owner_id: options.owner_id,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            settings: ProjectSettings::default(),
        };

        let mut tasks = Vec::new();
        let mut task_uid_map: HashMap<i32, Uuid> = HashMap::new();

        if let Some(ms_tasks) = &ms_project.tasks {
            for ms_task in &ms_tasks.task {
                if ms_task.is_null.unwrap_or(false) {
                    continue;
                }

                if ms_task.name.is_none() || ms_task.name.as_ref().map(|n| n.is_empty()).unwrap_or(true) {
                    stats.tasks_skipped += 1;
                    continue;
                }

                let task_id = Uuid::new_v4();
                task_uid_map.insert(ms_task.uid, task_id);

                let task_start = ms_task
                    .start
                    .as_ref()
                    .and_then(|s| parse_ms_date(s))
                    .unwrap_or(start_date);

                let task_end = ms_task
                    .finish
                    .as_ref()
                    .and_then(|s| parse_ms_date(s))
                    .unwrap_or(task_start);

                let duration_days = parse_ms_duration(&ms_task.duration).unwrap_or(1);

                let priority = match ms_task.priority.unwrap_or(500) {
                    0..=200 => TaskPriority::Low,
                    201..=400 => TaskPriority::Normal,
                    401..=700 => TaskPriority::High,
                    _ => TaskPriority::Critical,
                };

                let task_type = if ms_task.milestone.unwrap_or(false) {
                    TaskType::Milestone
                } else if ms_task.summary.unwrap_or(false) {
                    TaskType::Summary
                } else {
                    TaskType::Task
                };

                let task = ProjectTask {
                    id: task_id,
                    project_id: project.id,
                    parent_id: None,
                    name: ms_task.name.clone().unwrap_or_default(),
                    description: None,
                    task_type,
                    start_date: task_start,
                    end_date: task_end,
                    duration_days,
                    percent_complete: ms_task.percent_complete.unwrap_or(0) as u8,
                    status: if ms_task.percent_complete.unwrap_or(0) >= 100 {
                        TaskStatus::Completed
                    } else if ms_task.percent_complete.unwrap_or(0) > 0 {
                        TaskStatus::InProgress
                    } else {
                        TaskStatus::NotStarted
                    },
                    priority,
                    assigned_to: Vec::new(),
                    dependencies: Vec::new(),
                    estimated_hours: parse_ms_work(&ms_task.work),
                    actual_hours: None,
                    cost: ms_task.cost,
                    notes: ms_task.notes.clone(),
                    wbs: ms_task.wbs.clone().unwrap_or_default(),
                    outline_level: ms_task.outline_level.unwrap_or(1) as u32,
                    is_milestone: ms_task.milestone.unwrap_or(false),
                    is_summary: ms_task.summary.unwrap_or(false),
                    is_critical: ms_task.critical.unwrap_or(false),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };

                tasks.push(task);
                stats.tasks_imported += 1;
            }
        }

        if options.import_dependencies {
            if let Some(ms_tasks) = &ms_project.tasks {
                for ms_task in &ms_tasks.task {
                    if let Some(task_id) = task_uid_map.get(&ms_task.uid) {
                        for pred_link in &ms_task.predecessor_links {
                            if let Some(pred_id) = task_uid_map.get(&pred_link.predecessor_uid) {
                                let dep_type = match pred_link.link_type.unwrap_or(1) {
                                    0 => DependencyType::FinishToFinish,
                                    1 => DependencyType::FinishToStart,
                                    2 => DependencyType::StartToFinish,
                                    3 => DependencyType::StartToStart,
                                    _ => DependencyType::FinishToStart,
                                };

                                let lag_days = pred_link.link_lag.unwrap_or(0) / 4800;

                                if let Some(task) = tasks.iter_mut().find(|t| t.id == *task_id) {
                                    task.dependencies.push(TaskDependency {
                                        predecessor_id: *pred_id,
                                        dependency_type: dep_type,
                                        lag_days,
                                    });
                                    stats.dependencies_imported += 1;
                                }
                            } else {
                                warnings.push(ImportWarning {
                                    code: "PRED_NOT_FOUND".to_string(),
                                    message: format!(
                                        "Predecessor task UID {} not found for task {}",
                                        pred_link.predecessor_uid, ms_task.uid
                                    ),
                                    source_element: Some(format!("Task UID {}", ms_task.uid)),
                                    suggested_action: Some("Dependency will be skipped".to_string()),
                                });
                            }
                        }
                    }
                }
            }
        }

        let mut resources = Vec::new();
        let mut resource_uid_map: HashMap<i32, Uuid> = HashMap::new();

        if options.import_resources {
            if let Some(ms_resources) = &ms_project.resources {
                for ms_resource in &ms_resources.resource {
                    if ms_resource.is_null.unwrap_or(false) {
                        continue;
                    }

                    if ms_resource.name.is_none()
                        || ms_resource.name.as_ref().map(|n| n.is_empty()).unwrap_or(true)
                    {
                        continue;
                    }

                    let resource_id = Uuid::new_v4();
                    resource_uid_map.insert(ms_resource.uid, resource_id);

                    let resource_type = match ms_resource.resource_type.unwrap_or(1) {
                        0 => ResourceType::Material,
                        1 => ResourceType::Work,
                        2 => ResourceType::Cost,
                        _ => ResourceType::Work,
                    };

                    let resource = Resource {
                        id: resource_id,
                        project_id: project.id,
                        user_id: options
                            .map_users
                            .get(ms_resource.name.as_ref().unwrap_or(&String::new()))
                            .copied(),
                        name: ms_resource.name.clone().unwrap_or_default(),
                        resource_type,
                        email: ms_resource.email_address.clone(),
                        max_units: ms_resource.max_units.unwrap_or(1.0) as f32,
                        standard_rate: Some(ms_resource.standard_rate.unwrap_or(options.default_resource_rate)),
                        overtime_rate: Some(ms_resource.overtime_rate.unwrap_or(0.0)),
                        cost_per_use: Some(ms_resource.cost_per_use.unwrap_or(0.0)),
                        calendar_id: None,
                        created_at: Utc::now(),
                    };

                    resources.push(resource);
                    stats.resources_imported += 1;
                }
            }
        }

        let mut assignments = Vec::new();

        if options.import_assignments {
            if let Some(ms_assignments) = &ms_project.assignments {
                for ms_assignment in &ms_assignments.assignment {
                    let task_id = task_uid_map.get(&ms_assignment.task_uid);
                    let resource_id = resource_uid_map.get(&ms_assignment.resource_uid);

                    match (task_id, resource_id) {
                        (Some(tid), Some(rid)) => {
                            let assignment_start = ms_assignment
                                .start
                                .as_ref()
                                .and_then(|s| parse_ms_date(s))
                                .unwrap_or(start_date);

                            let assignment_end = ms_assignment
                                .finish
                                .as_ref()
                                .and_then(|s| parse_ms_date(s))
                                .unwrap_or(assignment_start);

                            let work_hours = parse_ms_work(&ms_assignment.work).unwrap_or(0.0);

                            let assignment = ResourceAssignment {
                                id: Uuid::new_v4(),
                                task_id: *tid,
                                resource_id: *rid,
                                units: ms_assignment.units.unwrap_or(1.0) as f32,
                                work_hours,
                                start_date: assignment_start,
                                end_date: assignment_end,
                                cost: ms_assignment.cost.unwrap_or(0.0),
                            };

                            if let Some(task) = tasks.iter_mut().find(|t| t.id == *tid) {
                                if !task.assigned_to.contains(rid) {
                                    task.assigned_to.push(*rid);
                                }
                            }

                            assignments.push(assignment);
                            stats.assignments_imported += 1;
                        }
                        _ => {
                            warnings.push(ImportWarning {
                                code: "ASSIGNMENT_REF_MISSING".to_string(),
                                message: format!(
                                    "Assignment {} references missing task or resource",
                                    ms_assignment.uid
                                ),
                                source_element: Some(format!("Assignment UID {}", ms_assignment.uid)),
                                suggested_action: Some("Assignment will be skipped".to_string()),
                            });
                        }
                    }
                }
            }
        }

        self.resolve_task_hierarchy(&mut tasks);

        Ok(ImportResult {
            project,
            tasks,
            resources,
            assignments,
            warnings,
            errors,
            stats,
        })
    }
}
