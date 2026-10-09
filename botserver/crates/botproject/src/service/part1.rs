use super::*;

impl ProjectService {
    pub fn new() -> Self {
        Self::with_optional_pool(None)
    }

    /// DB-backed: state is written through to `project_snapshots` on every
    /// mutation and loaded back on [`ProjectService::load_from_db`].
    pub fn with_pool(pool: DbPool) -> Self {
        Self::with_optional_pool(Some(pool))
    }

    pub(super) fn with_optional_pool(pool: Option<DbPool>) -> Self {
        Self {
            projects: Arc::new(RwLock::new(HashMap::new())),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            resources: Arc::new(RwLock::new(HashMap::new())),
            assignments: Arc::new(RwLock::new(HashMap::new())),
            pool,
        }
    }

    /// Hydrate the in-memory store from any previously persisted snapshots.
    /// Missing rows / parse failures are logged and skipped, never fatal.
    pub async fn load_from_db(&self) {
        let Some(pool) = self.pool.clone() else { return; };

        #[derive(QueryableByName)]
        struct Row {
            #[diesel(sql_type = diesel::sql_types::Jsonb)]
            payload: serde_json::Value,
        }

        let loaded = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            diesel::sql_query("SELECT payload FROM project_snapshots")
                .load::<Row>(&mut conn)
                .map_err(|e| e.to_string())
        })
        .await;

        let rows = match loaded {
            Ok(Ok(rows)) => rows,
            Ok(Err(e)) => {
                warn!("project load failed: {e}");
                return;
            }
            Err(e) => {
                warn!("project load join failed: {e}");
                return;
            }
        };

        for row in rows {
            let snap: OrgSnapshot = match serde_json::from_value(row.payload) {
                Ok(s) => s,
                Err(e) => {
                    warn!("project snapshot parse failed: {e}");
                    continue;
                }
            };
            {
                let mut projects = self.projects.write().await;
                for p in snap.projects {
                    projects.entry(p.id).or_insert(p);
                }
            }
            {
                let mut tasks = self.tasks.write().await;
                for t in snap.tasks {
                    tasks.entry(t.id).or_insert(t);
                }
            }
            {
                let mut resources = self.resources.write().await;
                for r in snap.resources {
                    resources.entry(r.id).or_insert(r);
                }
            }
            {
                let mut assignments = self.assignments.write().await;
                for a in snap.assignments {
                    assignments.entry(a.id).or_insert(a);
                }
            }
        }
    }

    /// Persist the whole in-memory store as one JSONB row per organization.
    /// Called after every mutation; failures are logged, never fatal (the
    /// in-memory store stays authoritative for the live UI).
    pub async fn persist_all(&self) {
        let Some(pool) = self.pool.clone() else { return; };

        let grouped: HashMap<Uuid, OrgSnapshot> = {
            let projects = self.projects.read().await;
            let tasks = self.tasks.read().await;
            let resources = self.resources.read().await;
            let assignments = self.assignments.read().await;

            let mut by_org: HashMap<Uuid, OrgSnapshot> = HashMap::new();
            let mut project_org: HashMap<Uuid, Uuid> = HashMap::new();
            for p in projects.values() {
                project_org.insert(p.id, p.organization_id);
                by_org
                    .entry(p.organization_id)
                    .or_default()
                    .projects
                    .push(p.clone());
            }
            let mut task_org: HashMap<Uuid, Uuid> = HashMap::new();
            for t in tasks.values() {
                if let Some(org) = project_org.get(&t.project_id) {
                    task_org.insert(t.id, *org);
                    by_org.entry(*org).or_default().tasks.push(t.clone());
                }
            }
            for r in resources.values() {
                if let Some(org) = project_org.get(&r.project_id) {
                    by_org.entry(*org).or_default().resources.push(r.clone());
                }
            }
            for a in assignments.values() {
                if let Some(org) = task_org.get(&a.task_id) {
                    by_org.entry(*org).or_default().assignments.push(a.clone());
                }
            }
            by_org
        };

        let rows: Vec<(Uuid, serde_json::Value)> = grouped
            .into_iter()
            .filter_map(|(org, snap)| serde_json::to_value(&snap).ok().map(|v| (org, v)))
            .collect();

        let res = tokio::task::spawn_blocking(move || {
            let mut conn = match pool.get() {
                Ok(c) => c,
                Err(e) => return Err(e.to_string()),
            };
            for (org, payload) in rows {
                diesel::sql_query(
                    "INSERT INTO project_snapshots (org_id, payload, updated_at) \
                     VALUES ($1, $2, NOW()) \
                     ON CONFLICT (org_id) DO UPDATE SET payload = EXCLUDED.payload, updated_at = NOW()",
                )
                .bind::<diesel::sql_types::Uuid, _>(org)
                .bind::<diesel::sql_types::Jsonb, _>(payload)
                .execute(&mut conn)
                .map_err(|e| e.to_string())?;
            }
            Ok(())
        })
        .await;

        if let Err(e) = res {
            warn!("project persist_all failed: {e}");
        }
    }

    pub async fn create_project(&self, project: Project) -> Project {
        {
            let mut projects = self.projects.write().await;
            projects.insert(project.id, project.clone());
        }
        self.persist_all().await;
        project
    }

    pub async fn get_project(&self, project_id: Uuid) -> Option<Project> {
        let projects = self.projects.read().await;
        projects.get(&project_id).cloned()
    }

    pub async fn get_projects_for_organization(&self, org_id: Uuid) -> Vec<Project> {
        let projects = self.projects.read().await;
        projects
            .values()
            .filter(|p| p.organization_id == org_id)
            .cloned()
            .collect()
    }

    pub async fn get_all_projects(&self) -> Vec<Project> {
        let projects = self.projects.read().await;
        projects.values().cloned().collect()
    }

    pub async fn update_project(&self, project: Project) -> Option<Project> {
        let updated = {
            let mut projects = self.projects.write().await;
            match projects.get_mut(&project.id) {
                Some(slot) => {
                    *slot = project.clone();
                    Some(project)
                }
                None => None,
            }
        };
        self.persist_all().await;
        updated
    }

    pub async fn delete_project(&self, project_id: Uuid) -> bool {
        let removed = {
            let mut projects = self.projects.write().await;
            let mut tasks = self.tasks.write().await;
            let mut resources = self.resources.write().await;

            tasks.retain(|_, t| t.project_id != project_id);
            resources.retain(|_, r| r.project_id != project_id);
            projects.remove(&project_id).is_some()
        };
        self.persist_all().await;
        removed
    }

    pub async fn create_task(&self, task: ProjectTask) -> ProjectTask {
        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(task.id, task.clone());
        }
        self.persist_all().await;
        task
    }

    pub async fn get_task(&self, task_id: Uuid) -> Option<ProjectTask> {
        let tasks = self.tasks.read().await;
        tasks.get(&task_id).cloned()
    }

    pub async fn get_tasks_for_project(&self, project_id: Uuid) -> Vec<ProjectTask> {
        let tasks = self.tasks.read().await;
        let mut project_tasks: Vec<ProjectTask> = tasks
            .values()
            .filter(|t| t.project_id == project_id)
            .cloned()
            .collect();
        project_tasks.sort_by(|a, b| a.wbs.cmp(&b.wbs));
        project_tasks
    }

    pub async fn update_task(&self, task: ProjectTask) -> Option<ProjectTask> {
        let updated = {
            let mut tasks = self.tasks.write().await;
            match tasks.get_mut(&task.id) {
                Some(slot) => {
                    *slot = task.clone();
                    Some(task)
                }
                None => None,
            }
        };
        self.persist_all().await;
        updated
    }

    pub async fn delete_task(&self, task_id: Uuid) -> bool {
        let removed = {
            let mut tasks = self.tasks.write().await;
            let mut assignments = self.assignments.write().await;

            assignments.retain(|_, a| a.task_id != task_id);
            tasks.remove(&task_id).is_some()
        };
        self.persist_all().await;
        removed
    }
}
