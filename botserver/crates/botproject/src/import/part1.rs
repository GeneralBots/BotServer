use super::*;

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            format: ImportFormat::MsProjectXml,
            organization_id: Uuid::nil(),
            owner_id: Uuid::nil(),
            import_resources: true,
            import_assignments: true,
            import_dependencies: true,
            import_custom_fields: false,
            map_users: HashMap::new(),
            default_resource_rate: 50.0,
            preserve_task_ids: false,
            conflict_resolution: ConflictResolution::CreateNew,
        }
    }
}
