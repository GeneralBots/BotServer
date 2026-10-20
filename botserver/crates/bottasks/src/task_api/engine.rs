use std::sync::Arc;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::state::TasksState;
use crate::types::{NewTask, Task, UpdateTaskRequest};

pub struct TaskEngine {
    state: Arc<TasksState>,
}

impl TaskEngine {
    pub fn new(state: Arc<TasksState>) -> Self {
        Self { state }
    }

    pub fn create_task(
        &self,
        task_branch_id: Uuid,
        task_title: &str,
        task_description: Option<&str>,
    ) -> Result<Task, String> {
        use crate::schema::tasks::dsl::*;

        let mut conn = self
            .state
            .pool
            .get()
            .map_err(|e| format!("Pool error: {}", e))?;

        let new_task = NewTask {
            id: Uuid::new_v4(),
            branch_id: task_branch_id,
            title: task_title.to_string(),
            description: task_description.map(|s| s.to_string()),
            status: "todo".to_string(),
            priority: "medium".to_string(),
            assignee_id: None,
            due_date: None,
            completed_at: None,
            parent_id: None,
        };

        diesel::insert_into(tasks)
            .values(&new_task)
            .execute(&mut conn)
            .map_err(|e| format!("Insert error: {}", e))?;

        tasks
            .find(new_task.id)
            .first::<Task>(&mut conn)
            .map_err(|e| format!("Fetch error: {}", e))
    }

    pub fn list_tasks(&self, filter_branch_id: Option<Uuid>) -> Result<Vec<Task>, String> {
        use crate::schema::tasks::dsl::*;

        let mut conn = self
            .state
            .pool
            .get()
            .map_err(|e| format!("Pool error: {}", e))?;

        let query = tasks.into_boxed();
        match filter_branch_id {
            Some(bid) => query
                .filter(branch_id.eq(bid))
                .load::<Task>(&mut conn)
                .map_err(|e| format!("Query error: {}", e)),
            None => query
                .load::<Task>(&mut conn)
                .map_err(|e| format!("Query error: {}", e)),
        }
    }

    pub fn get_task(&self, task_id: Uuid, task_branch_id: Uuid) -> Result<Task, String> {
        use crate::schema::tasks::dsl::*;

        let mut conn = self
            .state
            .pool
            .get()
            .map_err(|e| format!("Pool error: {}", e))?;

        tasks
            .filter(id.eq(task_id).and(branch_id.eq(task_branch_id)))
            .first::<Task>(&mut conn)
            .map_err(|e| format!("Query error: {}", e))
    }

    pub fn update_task_status(&self, task_id: Uuid, task_branch_id: Uuid, new_status: &str) -> Result<(), String> {
        use crate::schema::tasks::dsl::*;

        let mut conn = self
            .state
            .pool
            .get()
            .map_err(|e| format!("Pool error: {}", e))?;

        diesel::update(tasks.filter(id.eq(task_id).and(branch_id.eq(task_branch_id))))
            .set((status.eq(new_status.to_string()), updated_at.eq(Utc::now())))
            .execute(&mut conn)
            .map_err(|e| format!("Update error: {}", e))?;

        Ok(())
    }

    /// Applies a partial update to a task, scoped by tenant branch (issue #877).
    ///
    /// The task is first fetched inside the branch filter (which both proves it
    /// exists and enforces tenant isolation), then each provided field is merged
    /// over the current value. The final `UPDATE` re-applies the branch filter so
    /// a caller can never modify a row that does not belong to their branch.
    pub fn update_task(
        &self,
        task_id: Uuid,
        task_branch_id: Uuid,
        update: &UpdateTaskRequest,
    ) -> Result<Task, String> {
        use crate::schema::tasks::dsl::*;

        let mut conn = self
            .state
            .pool
            .get()
            .map_err(|e| format!("Pool error: {}", e))?;

        let current = tasks
            .filter(id.eq(task_id).and(branch_id.eq(task_branch_id)))
            .first::<Task>(&mut conn)
            .map_err(|e| format!("Query error: {}", e))?;

        let new_title = match &update.title {
            Some(t) if !t.trim().is_empty() => t.clone(),
            _ => current.title.clone(),
        };
        let new_description = match &update.description {
            None => current.description.clone(),
            Some(inner) => inner.clone(),
        };
        let new_priority = update.priority.clone().unwrap_or_else(|| current.priority.clone());
        let new_assignee_id = match &update.assignee_id {
            None => current.assignee_id,
            Some(inner) => *inner,
        };
        let new_due_date = match &update.due_date {
            None => current.due_date,
            Some(inner) => *inner,
        };
        let new_parent_id = match &update.parent_id {
            None => current.parent_id,
            Some(inner) => *inner,
        };

        diesel::update(tasks.filter(id.eq(task_id).and(branch_id.eq(task_branch_id))))
            .set((
                title.eq(new_title),
                description.eq(new_description),
                priority.eq(new_priority),
                assignee_id.eq(new_assignee_id),
                due_date.eq(new_due_date),
                parent_id.eq(new_parent_id),
                updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .map_err(|e| format!("Update error: {}", e))?;

        tasks
            .filter(id.eq(task_id).and(branch_id.eq(task_branch_id)))
            .first::<Task>(&mut conn)
            .map_err(|e| format!("Fetch error: {}", e))
    }

    /// Marks a task completed, recording the completion timestamp (issue #877).
    pub fn complete_task(&self, task_id: Uuid, task_branch_id: Uuid) -> Result<Task, String> {
        use crate::schema::tasks::dsl::*;

        let mut conn = self
            .state
            .pool
            .get()
            .map_err(|e| format!("Pool error: {}", e))?;

        let now = Utc::now();
        diesel::update(tasks.filter(id.eq(task_id).and(branch_id.eq(task_branch_id))))
            .set((
                status.eq("completed".to_string()),
                completed_at.eq(Some(now)),
                updated_at.eq(now),
            ))
            .execute(&mut conn)
            .map_err(|e| format!("Update error: {}", e))?;

        tasks
            .filter(id.eq(task_id).and(branch_id.eq(task_branch_id)))
            .first::<Task>(&mut conn)
            .map_err(|e| format!("Fetch error: {}", e))
    }

    /// Reopens a completed task, clearing the completion timestamp (issue #877).
    pub fn reopen_task(&self, task_id: Uuid, task_branch_id: Uuid) -> Result<Task, String> {
        use crate::schema::tasks::dsl::*;

        let mut conn = self
            .state
            .pool
            .get()
            .map_err(|e| format!("Pool error: {}", e))?;

        diesel::update(tasks.filter(id.eq(task_id).and(branch_id.eq(task_branch_id))))
            .set((
                status.eq("pending".to_string()),
                completed_at.eq(None::<DateTime<Utc>>),
                updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .map_err(|e| format!("Update error: {}", e))?;

        tasks
            .filter(id.eq(task_id).and(branch_id.eq(task_branch_id)))
            .first::<Task>(&mut conn)
            .map_err(|e| format!("Fetch error: {}", e))
    }

    pub fn delete_task(&self, task_id: Uuid, task_branch_id: Uuid) -> Result<(), String> {
        use crate::schema::tasks::dsl::*;

        let mut conn = self
            .state
            .pool
            .get()
            .map_err(|e| format!("Pool error: {}", e))?;

        diesel::delete(tasks.filter(id.eq(task_id).and(branch_id.eq(task_branch_id))))
            .execute(&mut conn)
            .map_err(|e| format!("Delete error: {}", e))?;

        Ok(())
    }
}
