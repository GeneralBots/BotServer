use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

use super::tasks_service_helpers;
use super::tasks_types::*;

#[derive(Debug, Clone)]
pub enum TasksIntegrationError {
    DatabaseError(String),
    ContactNotFound,
    TaskNotFound,
    AlreadyAssigned,
    NotAssigned,
    Unauthorized,
    InvalidInput(String),
}

impl std::fmt::Display for TasksIntegrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError(e) => write!(f, "Database error: {e}"),
            Self::ContactNotFound => write!(f, "Contact not found"),
            Self::TaskNotFound => write!(f, "Task not found"),
            Self::AlreadyAssigned => write!(f, "Contact already assigned"),
            Self::NotAssigned => write!(f, "Contact not assigned"),
            Self::Unauthorized => write!(f, "Unauthorized"),
            Self::InvalidInput(e) => write!(f, "Invalid input: {e}"),
        }
    }
}

impl std::error::Error for TasksIntegrationError {}

impl axum::response::IntoResponse for TasksIntegrationError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        let status = match &self {
            Self::ContactNotFound | Self::TaskNotFound => StatusCode::NOT_FOUND,
            Self::AlreadyAssigned | Self::NotAssigned => StatusCode::CONFLICT,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::InvalidInput(_) => StatusCode::BAD_REQUEST,
            Self::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, axum::Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&self, "botcontacts::tasks_service", None) }))).into_response()
    }
}

pub struct TasksIntegrationService {
    db_pool: crate::DbPool,
}

impl TasksIntegrationService {
    pub fn new(pool: crate::DbPool) -> Self {
        Self { db_pool: pool }
    }

    pub async fn assign_contact_to_task(
        &self,
        organization_id: Uuid,
        task_id: Uuid,
        request: &AssignContactRequest,
        assigned_by: Uuid,
    ) -> Result<TaskContact, TasksIntegrationError> {
        self.verify_contact(organization_id, request.contact_id).await?;
        self.verify_task(organization_id, task_id).await?;

        if self.is_contact_assigned(task_id, request.contact_id).await? {
            return Err(TasksIntegrationError::AlreadyAssigned);
        }

        let id = Uuid::new_v4();
        let now = Utc::now();
        let role = request.role.clone().unwrap_or_default();

        self.create_task_contact_assignment(TaskAssignmentParams {
            id, task_id, contact_id: request.contact_id,
            role: &role, assigned_by, notes: request.notes.as_deref(), assigned_at: now,
        }).await?;

        let notified = if request.send_notification.unwrap_or(true) {
            self.send_task_assignment_notification(task_id, request.contact_id).await.is_ok()
        } else { false };

        self.log_contact_activity(request.contact_id, TaskActivityType::Assigned, "Assigned to task", task_id).await?;

        Ok(TaskContact {
            id, task_id, contact_id: request.contact_id, role,
            assigned_at: now, assigned_by, notified,
            notified_at: if notified { Some(now) } else { None },
            notes: request.notes.clone(),
        })
    }

    pub async fn bulk_assign_contacts(
        &self,
        organization_id: Uuid,
        task_id: Uuid,
        request: &BulkAssignContactsRequest,
        assigned_by: Uuid,
    ) -> Result<Vec<TaskContact>, TasksIntegrationError> {
        let mut results = Vec::new();
        for assignment in &request.assignments {
            let assign_request = AssignContactRequest {
                contact_id: assignment.contact_id,
                role: assignment.role.clone(),
                send_notification: request.send_notification,
                notes: assignment.notes.clone(),
            };
            match self.assign_contact_to_task(organization_id, task_id, &assign_request, assigned_by).await {
                Ok(tc) => results.push(tc),
                Err(TasksIntegrationError::AlreadyAssigned) => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(results)
    }

    pub async fn unassign_contact_from_task(
        &self, organization_id: Uuid, task_id: Uuid, contact_id: Uuid,
    ) -> Result<(), TasksIntegrationError> {
        self.verify_contact(organization_id, contact_id).await?;
        self.verify_task(organization_id, task_id).await?;
        self.delete_task_contact_assignment(task_id, contact_id).await?;
        self.log_contact_activity(contact_id, TaskActivityType::Unassigned, "Unassigned from task", task_id).await?;
        Ok(())
    }

    pub async fn update_task_contact(
        &self, organization_id: Uuid, task_id: Uuid, contact_id: Uuid,
        request: &UpdateTaskContactRequest,
    ) -> Result<TaskContact, TasksIntegrationError> {
        self.verify_contact(organization_id, contact_id).await?;
        self.verify_task(organization_id, task_id).await?;

        let mut task_contact = self.get_task_contact(task_id, contact_id).await?;
        if let Some(role) = &request.role { task_contact.role = role.clone(); }
        if let Some(notes) = &request.notes { task_contact.notes = Some(notes.clone()); }

        self.update_task_contact_in_db(&task_contact).await?;
        Ok(task_contact)
    }

    pub async fn get_task_contacts(
        &self, organization_id: Uuid, task_id: Uuid, query: &TaskContactsQuery,
    ) -> Result<Vec<TaskContactWithDetails>, TasksIntegrationError> {
        self.verify_task(organization_id, task_id).await?;
        let contacts = self.fetch_task_contacts(task_id, query).await?;

        if query.include_contact_details.unwrap_or(true) {
            let mut results = Vec::new();
            for tc in contacts {
                if let Ok(contact) = self.get_contact_summary(tc.contact_id).await {
                    results.push(TaskContactWithDetails { task_contact: tc, contact });
                }
            }
            Ok(results)
        } else {
            Ok(contacts.into_iter().map(|tc| TaskContactWithDetails {
                contact: ContactSummary {
                    id: tc.contact_id, first_name: String::new(), last_name: String::new(),
                    email: None, phone: None, company: None, job_title: None, avatar_url: None,
                },
                task_contact: tc,
            }).collect())
        }
    }

    pub async fn get_contact_tasks(
        &self, organization_id: Uuid, contact_id: Uuid, query: &ContactTasksQuery,
    ) -> Result<ContactTasksResponse, TasksIntegrationError> {
        self.verify_contact(organization_id, contact_id).await?;
        let tasks = self.fetch_contact_tasks(contact_id, query).await?;
        let total_count = tasks.len() as u32;
        let now = Utc::now();
        let week_end = now + chrono::Duration::days(7);

        let mut by_status: HashMap<String, u32> = HashMap::new();
        let mut by_priority: HashMap<String, u32> = HashMap::new();
        let mut overdue_count = 0u32;
        let mut due_today_count = 0u32;
        let mut due_this_week_count = 0u32;

        for task in &tasks {
            *by_status.entry(task.task.status.clone()).or_insert(0) += 1;
            *by_priority.entry(task.task.priority.clone()).or_insert(0) += 1;
            if let Some(due_date) = task.task.due_date {
                if due_date < now && task.task.status != "completed" { overdue_count += 1; }
                else if due_date.date_naive() == now.date_naive() { due_today_count += 1; }
                else if due_date < week_end { due_this_week_count += 1; }
            }
        }

        Ok(ContactTasksResponse { tasks, total_count, by_status, by_priority, overdue_count, due_today_count, due_this_week_count })
    }

    pub async fn get_contact_task_stats(
        &self, organization_id: Uuid, contact_id: Uuid,
    ) -> Result<ContactTaskStats, TasksIntegrationError> {
        self.verify_contact(organization_id, contact_id).await?;
        Ok(ContactTaskStats {
            contact_id, total_tasks: 0, completed_tasks: 0, in_progress_tasks: 0,
            overdue_tasks: 0, completion_rate: 0.0, average_completion_time_days: None,
            tasks_by_role: HashMap::new(), recent_activity: vec![],
        })
    }

    pub async fn get_suggested_contacts(
        &self, organization_id: Uuid, task_id: Uuid, limit: Option<u32>,
    ) -> Result<Vec<SuggestedTaskContact>, TasksIntegrationError> {
        self.verify_task(organization_id, task_id).await?;
        let limit = limit.unwrap_or(10);
        let mut suggestions: Vec<SuggestedTaskContact> = Vec::new();

        let assigned = self.get_assigned_contact_ids(task_id).await?;
        let contacts = self.find_suggestion_contacts(&assigned, limit as usize).await?;
        for (contact, workload) in contacts {
            let reason = if workload.workload_level == WorkloadLevel::Low {
                TaskSuggestionReason::LowWorkload
            } else {
                TaskSuggestionReason::TeamMember
            };
            suggestions.push(SuggestedTaskContact { contact, reason, score: 0.7, workload });
        }

        suggestions.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        suggestions.truncate(limit as usize);
        Ok(suggestions)
    }

    pub async fn get_contact_workload(
        &self, organization_id: Uuid, contact_id: Uuid,
    ) -> Result<ContactWorkload, TasksIntegrationError> {
        self.verify_contact(organization_id, contact_id).await?;
        Ok(ContactWorkload { active_tasks: 0, high_priority_tasks: 0, overdue_tasks: 0, due_this_week: 0, workload_level: WorkloadLevel::Low })
    }

    pub async fn create_task_for_contact(
        &self, organization_id: Uuid, contact_id: Uuid,
        request: &CreateTaskForContactRequest, created_by: Uuid,
    ) -> Result<ContactTaskWithDetails, TasksIntegrationError> {
        self.verify_contact(organization_id, contact_id).await?;
        let task_id = Uuid::new_v4();
        let now = Utc::now();

        self.create_task_in_db(task_id, organization_id, &request.title, request.description.as_deref(), Some(created_by), request.due_date).await?;

        let assign_request = AssignContactRequest {
            contact_id, role: request.role.clone(), send_notification: request.send_notification, notes: None,
        };
        let task_contact = self.assign_contact_to_task(organization_id, task_id, &assign_request, created_by).await?;

        let task = TaskSummary {
            id: task_id, title: request.title.clone(), description: request.description.clone(),
            status: "todo".to_string(), priority: request.priority.clone().unwrap_or_else(|| "medium".to_string()),
            due_date: request.due_date, project_id: request.project_id, project_name: None,
            progress: 0, created_at: now, updated_at: now,
        };
        Ok(ContactTaskWithDetails { task_contact, task })
    }

    async fn send_task_assignment_notification(&self, _tid: Uuid, _cid: Uuid) -> Result<(), TasksIntegrationError> { Ok(()) }
    async fn log_contact_activity(&self, _cid: Uuid, _ty: TaskActivityType, _desc: &str, _tid: Uuid) -> Result<(), TasksIntegrationError> { Ok(()) }
    async fn verify_contact(&self, _org: Uuid, _cid: Uuid) -> Result<(), TasksIntegrationError> { Ok(()) }
    async fn verify_task(&self, _org: Uuid, _tid: Uuid) -> Result<(), TasksIntegrationError> { Ok(()) }
    async fn is_contact_assigned(&self, _tid: Uuid, _cid: Uuid) -> Result<bool, TasksIntegrationError> { Ok(false) }

    async fn create_task_contact_assignment(&self, _params: TaskAssignmentParams<'_>) -> Result<(), TasksIntegrationError> { Ok(()) }
    async fn delete_task_contact_assignment(&self, _tid: Uuid, _cid: Uuid) -> Result<(), TasksIntegrationError> { Ok(()) }

    async fn get_task_contact(&self, task_id: Uuid, contact_id: Uuid) -> Result<TaskContact, TasksIntegrationError> {
        Ok(TaskContact { id: Uuid::new_v4(), task_id, contact_id, role: TaskContactRole::Assignee, assigned_at: Utc::now(), assigned_by: Uuid::new_v4(), notified: false, notified_at: None, notes: None })
    }

    async fn update_task_contact_in_db(&self, task_contact: &TaskContact) -> Result<(), TasksIntegrationError> {
        let pool = self.db_pool.clone();
        let tc = task_contact.clone();
        tokio::task::spawn_blocking(move || tasks_service_helpers::update_task_contact_db(&pool, &tc)).await
            .map_err(|e| TasksIntegrationError::DatabaseError(e.to_string()))?
    }

    async fn fetch_task_contacts(&self, task_id: Uuid, _q: &TaskContactsQuery) -> Result<Vec<TaskContact>, TasksIntegrationError> {
        let pool = self.db_pool.clone();
        tokio::task::spawn_blocking(move || tasks_service_helpers::fetch_task_contacts_db(&pool, task_id)).await
            .map_err(|e| TasksIntegrationError::DatabaseError(e.to_string()))?
    }

    async fn fetch_contact_tasks(&self, contact_id: Uuid, query: &ContactTasksQuery) -> Result<Vec<ContactTaskWithDetails>, TasksIntegrationError> {
        let pool = self.db_pool.clone();
        let status = query.status.clone();
        tokio::task::spawn_blocking(move || tasks_service_helpers::fetch_contact_tasks_db(&pool, contact_id, status)).await
            .map_err(|e: tokio::task::JoinError| TasksIntegrationError::DatabaseError(e.to_string()))?
    }

    async fn get_contact_summary(&self, cid: Uuid) -> Result<ContactSummary, TasksIntegrationError> {
        Ok(ContactSummary { id: cid, first_name: String::new(), last_name: String::new(), email: None, phone: None, company: None, job_title: None, avatar_url: None })
    }

    async fn get_assigned_contact_ids(&self, task_id: Uuid) -> Result<Vec<Uuid>, TasksIntegrationError> {
        let pool = self.db_pool.clone();
        tokio::task::spawn_blocking(move || tasks_service_helpers::get_assigned_contact_ids_db(&pool, task_id)).await
            .map_err(|e| TasksIntegrationError::DatabaseError(e.to_string()))?
    }

    async fn find_suggestion_contacts(&self, exclude: &[Uuid], limit: usize) -> Result<Vec<(ContactSummary, ContactWorkload)>, TasksIntegrationError> {
        let pool = self.db_pool.clone();
        let exclude = exclude.to_vec();
        tokio::task::spawn_blocking(move || -> Result<Vec<(ContactSummary, ContactWorkload)>, TasksIntegrationError> {
            let contacts = tasks_service_helpers::query_contacts_for_suggestions(&pool, &exclude, limit)?;
            Ok(contacts.into_iter().map(|c| (c, ContactWorkload { active_tasks: 0, high_priority_tasks: 0, overdue_tasks: 0, due_this_week: 0, workload_level: WorkloadLevel::Low })).collect())
        }).await.map_err(|e| TasksIntegrationError::DatabaseError(e.to_string()))?
    }

    async fn create_task_in_db(&self, _tid: Uuid, _org: Uuid, _title: &str, _desc: Option<&str>, _assignee: Option<Uuid>, _due: Option<DateTime<Utc>>) -> Result<(), TasksIntegrationError> { Ok(()) }
}
