use super::*;

impl ProjectService {
    pub async fn add_dependency(
        &self,
        task_id: Uuid,
        predecessor_id: Uuid,
        dependency_type: DependencyType,
        lag_days: i32,
    ) -> Option<ProjectTask> {
        let result = {
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(&task_id) {
                let dependency = TaskDependency {
                    predecessor_id,
                    dependency_type,
                    lag_days,
                };
                // Re-linking the same predecessor updates the existing edge
                // instead of stacking duplicates (a task can only depend on a
                // predecessor once).
                if let Some(existing) = task
                    .dependencies
                    .iter_mut()
                    .find(|d| d.predecessor_id == predecessor_id)
                {
                    *existing = dependency;
                } else {
                    task.dependencies.push(dependency);
                }
                task.updated_at = Utc::now();
                Some(task.clone())
            } else {
                None
            }
        };
        self.persist_all().await;
        result
    }

    pub async fn remove_dependency(&self, task_id: Uuid, predecessor_id: Uuid) -> Option<ProjectTask> {
        let result = {
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(&task_id) {
                task.dependencies.retain(|d| d.predecessor_id != predecessor_id);
                task.updated_at = Utc::now();
                Some(task.clone())
            } else {
                None
            }
        };
        self.persist_all().await;
        result
    }

    pub async fn create_resource(&self, resource: Resource) -> Resource {
        {
            let mut resources = self.resources.write().await;
            resources.insert(resource.id, resource.clone());
        }
        self.persist_all().await;
        resource
    }

    pub async fn get_resources_for_project(&self, project_id: Uuid) -> Vec<Resource> {
        let resources = self.resources.read().await;
        resources
            .values()
            .filter(|r| r.project_id == project_id)
            .cloned()
            .collect()
    }

    pub async fn get_resource(&self, resource_id: Uuid) -> Option<Resource> {
        let resources = self.resources.read().await;
        resources.get(&resource_id).cloned()
    }

    pub async fn delete_resource(&self, resource_id: Uuid) -> bool {
        let removed = {
            let mut resources = self.resources.write().await;
            let mut assignments = self.assignments.write().await;
            assignments.retain(|_, a| a.resource_id != resource_id);
            resources.remove(&resource_id).is_some()
        };
        self.persist_all().await;
        removed
    }

    pub async fn get_assignments_for_task(&self, task_id: Uuid) -> Vec<ResourceAssignment> {
        let assignments = self.assignments.read().await;
        assignments
            .values()
            .filter(|a| a.task_id == task_id)
            .cloned()
            .collect()
    }

    pub async fn get_assignments_for_project(&self, project_id: Uuid) -> Vec<ResourceAssignment> {
        let tasks = self.tasks.read().await;
        let task_ids: std::collections::HashSet<Uuid> = tasks
            .values()
            .filter(|t| t.project_id == project_id)
            .map(|t| t.id)
            .collect();
        let assignments = self.assignments.read().await;
        assignments
            .values()
            .filter(|a| task_ids.contains(&a.task_id))
            .cloned()
            .collect()
    }

    /// Insert a whole imported snapshot (project + tasks + resources +
    /// assignments) in one shot and persist once. The import service already
    /// assigned ids and scoped `organization_id`/`owner_id`; this just stores
    /// them verbatim.
    pub async fn import_data(&self, result: ImportResult) -> ImportResult {
        {
            let mut projects = self.projects.write().await;
            projects.insert(result.project.id, result.project.clone());
            let mut tasks = self.tasks.write().await;
            for t in &result.tasks {
                tasks.insert(t.id, t.clone());
            }
            let mut resources = self.resources.write().await;
            for r in &result.resources {
                resources.insert(r.id, r.clone());
            }
            let mut assignments = self.assignments.write().await;
            for a in &result.assignments {
                assignments.insert(a.id, a.clone());
            }
        }
        self.persist_all().await;
        result
    }

    pub async fn assign_resource(
        &self,
        task_id: Uuid,
        resource_id: Uuid,
        units: f32,
        work_hours: f32,
    ) -> Option<ResourceAssignment> {
        let assignment = {
            let tasks = self.tasks.read().await;
            let resources = self.resources.read().await;

            let task = tasks.get(&task_id)?;
            let resource = resources.get(&resource_id)?;

            let cost = work_hours * resource.standard_rate.unwrap_or(0.0) as f32;

            ResourceAssignment {
                id: Uuid::new_v4(),
                task_id,
                resource_id,
                units,
                work_hours,
                start_date: task.start_date,
                end_date: task.end_date,
                cost: cost as f64,
            }
        };

        {
            let mut assignments = self.assignments.write().await;
            assignments.insert(assignment.id, assignment.clone());
        }
        self.persist_all().await;
        Some(assignment)
    }

    pub async fn update_task_progress(&self, task_id: Uuid, percent_complete: u8) -> Option<ProjectTask> {
        let result = {
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(&task_id) {
                task.percent_complete = percent_complete.min(100);
                task.status = if percent_complete == 0 {
                    TaskStatus::NotStarted
                } else if percent_complete == 100 {
                    TaskStatus::Completed
                } else {
                    TaskStatus::InProgress
                };
                task.updated_at = Utc::now();
                Some(task.clone())
            } else {
                None
            }
        };
        self.persist_all().await;
        result
    }
}
