pub(crate) use chrono::{DateTime, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskManifest {
    pub id: String,
    pub app_name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: ManifestStatus,
    pub current_status: CurrentStatus,
    pub sections: Vec<ManifestSection>,
    pub total_steps: u32,
    pub completed_steps: u32,
    pub runtime_seconds: u64,
    pub estimated_seconds: u64,
    pub terminal_output: Vec<TerminalLine>,
    pub processing_stats: ProcessingStats,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CurrentStatus {
    pub title: String,
    pub current_action: Option<String>,
    pub decision_point: Option<DecisionPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionPoint {
    pub step_current: u32,
    pub step_total: u32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ManifestStatus {
    #[default]
    Planning,
    Ready,
    Running,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestSection {
    pub id: String,
    pub name: String,
    pub section_type: SectionType,
    pub status: SectionStatus,
    pub current_step: u32,
    pub total_steps: u32,
    pub global_step_start: u32,
    pub duration_seconds: Option<u64>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub items: Vec<ManifestItem>,
    pub item_groups: Vec<ItemGroup>,
    pub children: Vec<ManifestSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SectionType {
    DatabaseModels,
    SchemaDesign,
    Tables,
    Files,
    Pages,
    Tools,
    Schedulers,
    Monitors,
    Validation,
    Deployment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum SectionStatus {
    #[default]
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestItem {
    pub id: String,
    pub name: String,
    pub item_type: ItemType,
    pub status: ItemStatus,
    pub details: Option<String>,
    pub duration_seconds: Option<u64>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemGroup {
    pub id: String,
    pub items: Vec<String>,
    pub status: ItemStatus,
    pub duration_seconds: Option<u64>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ItemType {
    Table,
    Field,
    Index,
    File,
    Page,
    Tool,
    Scheduler,
    Monitor,
    Config,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ItemStatus {
    #[default]
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalLine {
    pub timestamp: DateTime<Utc>,
    pub content: String,
    pub line_type: TerminalLineType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TerminalLineType {
    Info,
    Progress,
    Success,
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProcessingStats {
    pub data_points_processed: u64,
    pub processing_speed: f64,
    pub sources_per_min: f64,
    pub estimated_remaining_seconds: u64,
}

pub struct ManifestBuilder {
    manifest: TaskManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDefinition {
    pub name: String,
    pub fields: Vec<FieldDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub name: String,
    pub field_type: String,
    pub nullable: bool,
    #[serde(default)]
    pub is_key: bool,
    #[serde(default)]
    pub is_nullable: bool,
    #[serde(default)]
    pub default_value: Option<String>,
    #[serde(default)]
    pub reference_table: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDefinition {
    pub filename: String,
    pub size_estimate: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageDefinition {
    pub filename: String,
    pub page_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub filename: String,
    pub triggers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerDefinition {
    pub name: String,
    pub filename: String,
    pub schedule: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorDefinition {
    pub name: String,
    pub target: String,
}

pub struct ManifestData {
    pub tables: Vec<TableDefinition>,
    pub files: Vec<FileDefinition>,
    pub pages: Vec<PageDefinition>,
    pub tools: Vec<ToolDefinition>,
    pub schedulers: Vec<SchedulerDefinition>,
    pub monitors: Vec<MonitorDefinition>,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;

use part4::*;
pub use part6::*;
