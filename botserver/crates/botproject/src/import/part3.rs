use super::*;

impl ProjectImportService {
    pub fn new() -> Self {
        Self
    }

    pub fn import<R: Read>(
        &self,
        reader: R,
        options: ImportOptions,
    ) -> Result<ImportResult, String> {
        let start_time = std::time::Instant::now();

        let result = match options.format {
            ImportFormat::MsProjectXml => self.import_ms_project_xml(reader, &options),
            ImportFormat::MsProjectMpp => self.import_ms_project_mpp(reader, &options),
            ImportFormat::Csv => self.import_csv(reader, &options),
            ImportFormat::Json => self.import_json(reader, &options),
            ImportFormat::Jira => self.import_generic_json(reader, &options, "Jira"),
            ImportFormat::Asana => self.import_generic_json(reader, &options, "Asana"),
            ImportFormat::Trello => self.import_generic_json(reader, &options, "Trello"),
        };

        result.map(|mut r| {
            r.stats.import_duration_ms = start_time.elapsed().as_millis() as u64;
            r
        })
    }

    pub(super) fn import_generic_json<R: Read>(
        &self,
        mut reader: R,
        options: &ImportOptions,
        source_name: &str,
    ) -> Result<ImportResult, String> {
        let mut content = String::new();
        reader
            .read_to_string(&mut content)
            .map_err(|e| format!("Failed to read {source_name} content: {e}"))?;

        let project = Project {
            id: Uuid::new_v4(),
            organization_id: options.organization_id,
            name: format!("Imported {source_name} Project"),
            description: Some(format!("{source_name} import - manual task mapping may be required")),
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
                code: format!("{}_BASIC_IMPORT", source_name.to_uppercase()),
                message: format!("{source_name} import creates a basic project structure. Tasks may need manual adjustment."),
                source_element: None,
                suggested_action: Some("Review and adjust imported tasks as needed".to_string()),
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

    pub(super) fn resolve_task_hierarchy(&self, tasks: &mut [ProjectTask]) {
        let mut parent_map: HashMap<u32, Uuid> = HashMap::new();

        for task in tasks.iter() {
            parent_map.insert(task.outline_level, task.id);
        }

        for task in tasks.iter_mut() {
            if task.outline_level > 1 {
                if let Some(parent_id) = parent_map.get(&(task.outline_level - 1)) {
                    task.parent_id = Some(*parent_id);
                }
            }
        }
    }
}
