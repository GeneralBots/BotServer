use super::*;

impl ManifestBuilder {
    pub fn new(app_name: &str, description: &str) -> Self {
        Self {
            manifest: TaskManifest::new(app_name, description),
        }
    }

    pub fn with_tables(mut self, tables: Vec<TableDefinition>) -> Self {
        if tables.is_empty() {
            return self;
        }
        let mut db_section = ManifestSection::new("Database & Models", SectionType::DatabaseModels);
        let mut schema_section = ManifestSection::new("Database Schema Design", SectionType::SchemaDesign);
        for table in &tables {
            let field_count = table.fields.len();
            let field_names: Vec<String> = table.fields.iter().take(4).map(|f| f.name.clone()).collect();
            let fields_preview = if field_count > 4 {
                format!("{}, +{} more", field_names.join(", "), field_count - 4)
            } else {
                field_names.join(", ")
            };
            let mut item = ManifestItem::new(&table.name, ItemType::Table);
            item.details = Some(format!("{} fields: {}", field_count, fields_preview));
            schema_section.add_item(item);
        }
        db_section.add_child(schema_section);
        self.manifest.add_section(db_section);
        self
    }

    pub fn with_files(mut self, files: Vec<FileDefinition>) -> Self {
        if files.is_empty() {
            return self;
        }
        let mut files_section = ManifestSection::new("Files", SectionType::Files);
        let html_files: Vec<_> = files.iter().filter(|f| f.filename.ends_with(".html")).collect();
        let css_files: Vec<_> = files.iter().filter(|f| f.filename.ends_with(".css")).collect();
        let js_files: Vec<_> = files.iter().filter(|f| f.filename.ends_with(".js")).collect();
        if !html_files.is_empty() {
            let mut pages_child = ManifestSection::new("HTML Pages", SectionType::Pages);
            for file in &html_files {
                pages_child.add_item(ManifestItem::new(&file.filename, ItemType::Page));
            }
            files_section.add_child(pages_child);
        }
        if !css_files.is_empty() {
            let mut styles_child = ManifestSection::new("Stylesheets", SectionType::Files);
            for file in &css_files {
                styles_child.add_item(ManifestItem::new(&file.filename, ItemType::File));
            }
            files_section.add_child(styles_child);
        }
        if !js_files.is_empty() {
            let mut scripts_child = ManifestSection::new("Scripts", SectionType::Files);
            for file in &js_files {
                scripts_child.add_item(ManifestItem::new(&file.filename, ItemType::File));
            }
            files_section.add_child(scripts_child);
        }
        self.manifest.add_section(files_section);
        self
    }

    pub fn with_pages(self, _pages: Vec<PageDefinition>) -> Self {
        self
    }

    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        if tools.is_empty() {
            return self;
        }
        let mut tools_section = ManifestSection::new("Tools & Automation", SectionType::Tools);
        let mut automation_child = ManifestSection::new("BASIC Scripts", SectionType::Tools);
        for tool in tools {
            automation_child.add_item(ManifestItem::new(&tool.name, ItemType::Tool).with_details(&tool.filename));
        }
        tools_section.add_child(automation_child);
        self.manifest.add_section(tools_section);
        self
    }

    pub fn with_schedulers(mut self, schedulers: Vec<SchedulerDefinition>) -> Self {
        if schedulers.is_empty() {
            return self;
        }
        let mut sched_section = ManifestSection::new("Scheduled Tasks", SectionType::Schedulers);
        let mut cron_child = ManifestSection::new("Cron Jobs", SectionType::Schedulers);
        for scheduler in schedulers {
            cron_child.add_item(ManifestItem::new(&scheduler.name, ItemType::Scheduler).with_details(&scheduler.schedule));
        }
        sched_section.add_child(cron_child);
        self.manifest.add_section(sched_section);
        self
    }

    pub fn with_monitors(mut self, monitors: Vec<MonitorDefinition>) -> Self {
        if monitors.is_empty() {
            return self;
        }
        let mut mon_section = ManifestSection::new("Monitoring", SectionType::Monitors);
        let mut alerts_child = ManifestSection::new("Alert Rules", SectionType::Monitors);
        for monitor in monitors {
            alerts_child.add_item(ManifestItem::new(&monitor.name, ItemType::Monitor).with_details(&monitor.target));
        }
        mon_section.add_child(alerts_child);
        self.manifest.add_section(mon_section);
        self
    }

    pub fn with_estimated_time(mut self, seconds: u64) -> Self {
        self.manifest.estimated_seconds = seconds;
        self
    }

    pub fn build(mut self) -> TaskManifest {
        self.manifest.status = ManifestStatus::Ready;
        self.manifest
    }
}
