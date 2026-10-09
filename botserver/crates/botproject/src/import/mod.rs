pub(crate) use chrono::Utc;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::io::Read;
pub(crate) use uuid::Uuid;
pub(crate) use super::{ DependencyType, Project, ProjectSettings, ProjectStatus, ProjectTask, Resource, ResourceAssignment, ResourceType, TaskDependency, TaskPriority, TaskStatus, TaskType, };

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportFormat {
    MsProjectXml,
    MsProjectMpp,
    Csv,
    Json,
    Jira,
    Asana,
    Trello,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportOptions {
    pub format: ImportFormat,
    pub organization_id: Uuid,
    pub owner_id: Uuid,
    pub import_resources: bool,
    pub import_assignments: bool,
    pub import_dependencies: bool,
    pub import_custom_fields: bool,
    pub map_users: HashMap<String, Uuid>,
    pub default_resource_rate: f64,
    pub preserve_task_ids: bool,
    pub conflict_resolution: ConflictResolution,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    Skip,
    Overwrite,
    CreateNew,
    Merge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub project: Project,
    pub tasks: Vec<ProjectTask>,
    pub resources: Vec<Resource>,
    pub assignments: Vec<ResourceAssignment>,
    pub warnings: Vec<ImportWarning>,
    pub errors: Vec<ImportError>,
    pub stats: ImportStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportStats {
    pub tasks_imported: u32,
    pub tasks_skipped: u32,
    pub resources_imported: u32,
    pub dependencies_imported: u32,
    pub assignments_imported: u32,
    pub custom_fields_imported: u32,
    pub import_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportWarning {
    pub code: String,
    pub message: String,
    pub source_element: Option<String>,
    pub suggested_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportError {
    pub code: String,
    pub message: String,
    pub source_element: Option<String>,
    pub fatal: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct MsProjectXml {
    #[serde(rename = "Name", default)]
    name: Option<String>,
    #[serde(rename = "Title", default)]
    title: Option<String>,
    #[serde(rename = "StartDate", default)]
    start_date: Option<String>,
    #[serde(rename = "FinishDate", default)]
    finish_date: Option<String>,
    #[serde(rename = "Tasks", default)]
    tasks: Option<MsProjectTasks>,
    #[serde(rename = "Resources", default)]
    resources: Option<MsProjectResources>,
    #[serde(rename = "Assignments", default)]
    assignments: Option<MsProjectAssignments>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct MsProjectTasks {
    #[serde(rename = "Task", default)]
    task: Vec<MsProjectTask>,
}

#[derive(Debug, Clone, Deserialize)]
struct MsProjectTask {
    #[serde(rename = "UID", default)]
    uid: i32,
    #[serde(rename = "Name", default)]
    name: Option<String>,
    #[serde(rename = "IsNull", default)]
    is_null: Option<bool>,
    #[serde(rename = "WBS", default)]
    wbs: Option<String>,
    #[serde(rename = "OutlineLevel", default)]
    outline_level: Option<i32>,
    #[serde(rename = "Priority", default)]
    priority: Option<i32>,
    #[serde(rename = "Start", default)]
    start: Option<String>,
    #[serde(rename = "Finish", default)]
    finish: Option<String>,
    #[serde(rename = "Duration", default)]
    duration: Option<String>,
    #[serde(rename = "Work", default)]
    work: Option<String>,
    #[serde(rename = "PercentComplete", default)]
    percent_complete: Option<i32>,
    #[serde(rename = "Cost", default)]
    cost: Option<f64>,
    #[serde(rename = "Milestone", default)]
    milestone: Option<bool>,
    #[serde(rename = "Summary", default)]
    summary: Option<bool>,
    #[serde(rename = "Critical", default)]
    critical: Option<bool>,
    #[serde(rename = "Notes", default)]
    notes: Option<String>,
    #[serde(rename = "PredecessorLink", default)]
    predecessor_links: Vec<MsPredecessorLink>,
}

#[derive(Debug, Clone, Deserialize)]
struct MsPredecessorLink {
    #[serde(rename = "PredecessorUID", default)]
    predecessor_uid: i32,
    #[serde(rename = "Type", default)]
    link_type: Option<i32>,
    #[serde(rename = "LinkLag", default)]
    link_lag: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct MsProjectResources {
    #[serde(rename = "Resource", default)]
    resource: Vec<MsProjectResource>,
}

#[derive(Debug, Clone, Deserialize)]
struct MsProjectResource {
    #[serde(rename = "UID", default)]
    uid: i32,
    #[serde(rename = "Name", default)]
    name: Option<String>,
    #[serde(rename = "Type", default)]
    resource_type: Option<i32>,
    #[serde(rename = "IsNull", default)]
    is_null: Option<bool>,
    #[serde(rename = "MaxUnits", default)]
    max_units: Option<f64>,
    #[serde(rename = "StandardRate", default)]
    standard_rate: Option<f64>,
    #[serde(rename = "OvertimeRate", default)]
    overtime_rate: Option<f64>,
    #[serde(rename = "CostPerUse", default)]
    cost_per_use: Option<f64>,
    #[serde(rename = "EmailAddress", default)]
    email_address: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct MsProjectAssignments {
    #[serde(rename = "Assignment", default)]
    assignment: Vec<MsProjectAssignment>,
}

#[derive(Debug, Clone, Deserialize)]
struct MsProjectAssignment {
    #[serde(rename = "UID", default)]
    uid: i32,
    #[serde(rename = "TaskUID", default)]
    task_uid: i32,
    #[serde(rename = "ResourceUID", default)]
    resource_uid: i32,
    #[serde(rename = "Units", default)]
    units: Option<f64>,
    #[serde(rename = "Work", default)]
    work: Option<String>,
    #[serde(rename = "Start", default)]
    start: Option<String>,
    #[serde(rename = "Finish", default)]
    finish: Option<String>,
    #[serde(rename = "Cost", default)]
    cost: Option<f64>,
}

#[derive(Default)]
pub struct ProjectImportService;

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;

use part2::*;
use part6::*;
