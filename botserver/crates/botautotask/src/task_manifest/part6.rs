use super::*;

pub fn create_manifest_from_llm_response(
    app_name: &str,
    description: &str,
    data: ManifestData,
) -> TaskManifest {
    let estimated_time = estimate_generation_time(&data.tables, &data.files, &data.tools, &data.schedulers);
    ManifestBuilder::new(app_name, description)
        .with_tables(data.tables)
        .with_files(data.files)
        .with_pages(data.pages)
        .with_tools(data.tools)
        .with_schedulers(data.schedulers)
        .with_estimated_time(estimated_time)
        .build()
}
pub(super) fn estimate_generation_time(
    tables: &[TableDefinition],
    files: &[FileDefinition],
    tools: &[ToolDefinition],
    schedulers: &[SchedulerDefinition],
) -> u64 {
    let table_time: u64 = tables.iter().map(|t| 5 + t.fields.len() as u64).sum();
    let file_time: u64 = files.len() as u64 * 3;
    let tool_time: u64 = tools.len() as u64 * 10;
    let sched_time: u64 = schedulers.len() as u64 * 5;
    table_time + file_time + tool_time + sched_time + 30
}
