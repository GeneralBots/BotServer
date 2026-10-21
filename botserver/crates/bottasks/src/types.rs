use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::{auto_tasks, tasks};

#[derive(Debug, Clone, Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = tasks)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub assignee_id: Option<Uuid>,
    pub reporter_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub due_date: Option<DateTime<Utc>>,
    pub tags: Option<Vec<Option<String>>>,
    pub dependencies: Option<Vec<Option<Uuid>>>,
    pub estimated_hours: Option<f64>,
    pub actual_hours: Option<f64>,
    pub progress: Option<i32>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub branch_id: Uuid,
    pub stage: String,
    pub parent_id: Option<Uuid>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = tasks)]
pub struct NewTask {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub assignee_id: Option<Uuid>,
    pub due_date: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub parent_id: Option<Uuid>,
}

#[derive(Debug, Clone, Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = auto_tasks)]
pub struct AutoTask {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub session_id: Option<Uuid>,
    pub title: String,
    pub intent: String,
    pub status: String,
    pub execution_mode: String,
    pub priority: String,
    pub plan_id: Option<Uuid>,
    pub basic_program: Option<String>,
    pub current_step: Option<i32>,
    pub total_steps: Option<i32>,
    pub progress: Option<f64>,
    pub step_results: Option<serde_json::Value>,
    pub manifest_json: Option<serde_json::Value>,
    pub error: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub branch_id: Uuid,
    pub name: String,
    pub schedule: Option<String>,
    pub task_type: String,
    pub config: Option<serde_json::Value>,
    pub is_active: bool,
    pub last_run_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = auto_tasks)]
pub struct NewAutoTask {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub bot_id: Uuid,
    pub name: String,
    pub schedule: Option<String>,
    pub task_type: String,
    pub config: Option<serde_json::Value>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub schedule: Option<String>,
    pub enabled: Option<bool>,
}

/// Partial update payload for a task (issue #877).
///
/// Semantics follow the double-`Option` pattern so the API can distinguish
/// three states for nullable columns:
/// * `None`        — leave the column unchanged,
/// * `Some(None)`  — clear the column (set it to `NULL`),
/// * `Some(value)` — set the column to `value`.
///
/// `title` is non-nullable, so `Some(title)` replaces it (blank titles are
/// ignored) and `None` leaves it unchanged.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub priority: Option<String>,
    pub assignee_id: Option<Option<Uuid>>,
    pub due_date: Option<Option<DateTime<Utc>>>,
    pub parent_id: Option<Option<Uuid>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskManifest {
    pub id: Uuid,
    pub title: String,
    pub status: String,
    pub total_steps: u32,
    pub completed_steps: u32,
    pub current_status: CurrentStatus,
    pub sections: Vec<ManifestSection>,
    pub items: Vec<ManifestItem>,
    pub processing_stats: ProcessingStats,
    pub terminal_output: Vec<TerminalLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentStatus {
    pub title: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestSection {
    pub title: String,
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestItem {
    pub name: String,
    pub steps: Vec<ManifestStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestStep {
    pub name: String,
    pub status: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingStats {
    pub total_tokens: u64,
    pub total_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalLine {
    pub text: String,
    pub line_type: Option<String>,
}

impl TaskManifest {
    pub fn new(title: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: title.to_string(),
            status: "pending".to_string(),
            total_steps: 0,
            completed_steps: 0,
            current_status: CurrentStatus {
                title: "Initialized".to_string(),
                detail: None,
            },
            sections: Vec::new(),
            items: Vec::new(),
            processing_stats: ProcessingStats {
                total_tokens: 0,
                total_cost: 0.0,
            },
            terminal_output: Vec::new(),
        }
    }

    pub fn add_item(&mut self, name: &str, steps: Vec<&str>) {
        let manifest_steps: Vec<ManifestStep> = steps
            .iter()
            .map(|s| ManifestStep {
                name: s.to_string(),
                status: "pending".to_string(),
                detail: None,
            })
            .collect();
        self.total_steps += manifest_steps.len() as u32;
        self.items.push(ManifestItem {
            name: name.to_string(),
            steps: manifest_steps,
        });
    }

    pub fn complete_step(&mut self, item_name: &str, step_name: &str) {
        if let Some(item) = self.items.iter_mut().find(|i| i.name == item_name) {
            if let Some(step) = item.steps.iter_mut().find(|s| s.name == step_name) {
                if step.status != "completed" {
                    step.status = "completed".to_string();
                    self.completed_steps += 1;
                    self.current_status.title = format!("Completed: {}", step_name);
                }
            }
        }
        if self.completed_steps >= self.total_steps && self.total_steps > 0 {
            self.status = "completed".to_string();
        }
    }

    pub fn fail_step(&mut self, item_name: &str, step_name: &str, detail: &str) {
        if let Some(item) = self.items.iter_mut().find(|i| i.name == item_name) {
            if let Some(step) = item.steps.iter_mut().find(|s| s.name == step_name) {
                step.status = "failed".to_string();
                step.detail = Some(detail.to_string());
                self.current_status.title = format!("Failed: {}", step_name);
                self.current_status.detail = Some(detail.to_string());
            }
        }
        self.status = "failed".to_string();
    }

    pub fn add_terminal_output(&mut self, text: &str, line_type: Option<&str>) {
        self.terminal_output.push(TerminalLine {
            text: text.to_string(),
            line_type: line_type.map(|s| s.to_string()),
        });
    }

    pub fn add_tokens(&mut self, tokens: u64, cost: f64) {
        self.processing_stats.total_tokens += tokens;
        self.processing_stats.total_cost += cost;
    }
}
