//! `projects::registry` — split per #1443 (AGENTS.md 450-line rule).

use super::*;

#[derive(Debug, Default, Deserialize)]
pub struct ListProjectsQuery {
    pub branch_id: Option<Uuid>,
    pub project_type: Option<String>,
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    /// List across every branch instead of scoping to `branch_id`.
    ///
    /// `branch_id: None` means "the nil branch" for org callers (the #1267
    /// fallback), so it cannot double as "unscoped" — that would expose other
    /// orgs' projects. Platform/service admins set this explicitly.
    #[serde(default)]
    pub all_branches: bool,
}

/// DB-backed project registry.
#[derive(Clone)]
pub struct ProjectRegistry {
    pub(crate) pool: DbPool,
}

impl ProjectRegistry {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// #1371 — read-only pool access for handlers composing registry-backed
    /// flows (website Run reuses the publish path against the same pool).
    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    pub(crate) fn conn(&self) -> Result<diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::PgConnection>>, String> {
        self.pool.get().map_err(|e| format!("db pool: {e}"))
    }

    /// Idempotent schema bootstrap (mirror of the 6.5.49 migration).
    pub fn ensure_schema(&self) -> Result<(), String> {
        let mut conn = self.conn()?;
        ensure_schema_sql(&mut conn, VIBE_PROJECTS_SCHEMA, "vibe_projects schema")?;
        Ok(())
    }

    pub fn create(&self, req: &CreateProjectRequest) -> Result<Project, String> {
        let mut conn = self.conn()?;
        let org_id = req.org_id.unwrap_or_else(Uuid::nil);
        let branch_id = req.branch_id.unwrap_or_else(Uuid::nil);

        // #828 — idempotent create: return the existing project instead of
        // failing on the unique (branch_id, name) constraint.
        let existing = diesel::sql_query(
            "SELECT id, org_id, branch_id, name, project_type, repository, framework, \
             custom_domain, source_control, status, environment, payload, created_at, updated_at \
             FROM vibe_projects WHERE branch_id = $1 AND name = $2",
        )
        .bind::<diesel::sql_types::Uuid, _>(branch_id)
        .bind::<diesel::sql_types::Text, _>(&req.name)
        .get_result::<ProjectRow>(&mut conn)
        .optional()
        .map_err(|e| format!("get project: {e}"))?;
        if let Some(row) = existing {
            return Ok(row.into_project());
        }

        // #1372 — reject unknown kinds instead of silently coercing them to
        // `apps` (which provisions a VM on Run).
        let project_type = ProjectKind::parse_strict(
            req.project_type.as_deref().unwrap_or("bot"),
        )?;
        let repository = req.repository.clone().unwrap_or_else(|| req.name.clone());
        let framework = req.framework.clone().unwrap_or_default();
        let custom_domain = req.custom_domain.clone().unwrap_or_default();
        // #1503 — git is the default source control (github = external
        // import, native = legacy rows only).
        let source_control = match req.source_control.as_deref() {
            Some("github") => "github",
            _ => "git",
        };
        let environment = req.environment.clone().unwrap_or_else(|| "development".to_string());
        // #1271 — github-mode projects carry their clone URL in the payload
        // so the create handler can clone the external repository into the
        // workspace (instead of seeding the built-in template).
        let mut payload = serde_json::json!({});
        if let Some(clone_url) = req.clone_url.as_deref() {
            if !clone_url.trim().is_empty() {
                payload["clone_url"] = serde_json::json!(clone_url);
            }
        }
        // #1445 G3 — the user's intent is stored in the payload so a later
        // re-seed of an empty workspace (run_project_app) scaffolds from the
        // REAL description instead of a generic brief.
        if let Some(desc) = req.description.as_deref() {
            if !desc.trim().is_empty() {
                payload["description"] = serde_json::json!(desc);
            }
        }

        let id = Uuid::new_v4();
        diesel::sql_query(
            "INSERT INTO vibe_projects \
             (id, org_id, branch_id, name, project_type, repository, framework, custom_domain, \
              source_control, status, environment, payload, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NOW(), NOW())",
        )
        .bind::<diesel::sql_types::Uuid, _>(id)
        .bind::<diesel::sql_types::Uuid, _>(org_id)
        .bind::<diesel::sql_types::Uuid, _>(branch_id)
        .bind::<diesel::sql_types::Text, _>(&req.name)
        .bind::<diesel::sql_types::Text, _>(project_type.as_str())
        .bind::<diesel::sql_types::Text, _>(&repository)
        .bind::<diesel::sql_types::Text, _>(&framework)
        .bind::<diesel::sql_types::Text, _>(&custom_domain)
        .bind::<diesel::sql_types::Text, _>(source_control)
        .bind::<diesel::sql_types::Text, _>("pending")
        .bind::<diesel::sql_types::Text, _>(&environment)
        .bind::<diesel::sql_types::Jsonb, _>(&payload)
        .execute(&mut conn)
        .map_err(|e| format!("insert project: {e}"))?;

        Ok(Project {
            id,
            org_id,
            branch_id,
            name: req.name.clone(),
            project_type: project_type.as_str().to_string(),
            repository,
            framework: req.framework.clone(),
            custom_domain: req.custom_domain.clone(),
            source_control: source_control.to_string(),
            status: "pending".to_string(),
            environment,
            payload,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    pub fn list(&self, query: &ListProjectsQuery) -> Result<Vec<Project>, String> {
        let mut conn = self.conn()?;
        let branch_id = query.branch_id.unwrap_or_else(Uuid::nil);
        let limit = query.limit.unwrap_or(100).min(500);
        let offset = query.offset.unwrap_or(0).max(0);

        let mut sql = String::from(
            "SELECT id, org_id, branch_id, name, project_type, repository, framework, \
             custom_domain, source_control, status, environment, payload, created_at, updated_at \
             FROM vibe_projects WHERE 1=1",
        );
        if !query.all_branches {
            sql.push_str(" AND branch_id = $1");
        }
        // $1 is the branch predicate only when it is present; without it the
        // optional filters shift down one slot, so the index is derived from
        // `binds.len()` rather than hardcoded.
        let base = if query.all_branches { 1 } else { 2 };
        let mut binds: Vec<String> = Vec::new();
        if let Some(ref pt) = query.project_type {
            sql.push_str(" AND project_type = $");
            sql.push_str(&(base + binds.len()).to_string());
            binds.push(pt.clone());
        }
        if let Some(ref st) = query.status {
            sql.push_str(" AND status = $");
            sql.push_str(&(base + binds.len()).to_string());
            binds.push(st.clone());
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT $");
        let limit_idx = base + binds.len();
        sql.push_str(&limit_idx.to_string());
        sql.push_str(" OFFSET $");
        sql.push_str(&(limit_idx + 1).to_string());

        let mut query_builder = diesel::sql_query(sql).into_boxed::<diesel::pg::Pg>();
        if !query.all_branches {
            query_builder = query_builder.bind::<diesel::sql_types::Uuid, _>(branch_id);
        }
        for b in &binds {
            query_builder = query_builder.bind::<diesel::sql_types::Text, _>(b.clone());
        }
        query_builder = query_builder
            .bind::<diesel::sql_types::BigInt, _>(limit)
            .bind::<diesel::sql_types::BigInt, _>(offset);

        query_builder
            .load::<ProjectRow>(&mut conn)
            .map(|rows| rows.into_iter().map(ProjectRow::into_project).collect())
            .map_err(|e| format!("list projects: {e}"))
    }

    pub fn get(&self, id: Uuid) -> Result<Option<Project>, String> {
        let mut conn = self.conn()?;
        let row = diesel::sql_query(
            "SELECT id, org_id, branch_id, name, project_type, repository, framework, \
             custom_domain, source_control, status, environment, payload, created_at, updated_at \
             FROM vibe_projects WHERE id = $1",
        )
        .bind::<diesel::sql_types::Uuid, _>(id)
        .get_result::<ProjectRow>(&mut conn)
        .optional()
        .map_err(|e| format!("get project: {e}"))?;
        Ok(row.map(ProjectRow::into_project))
    }

    /// Append a deployment record to the project payload (deploy history;
    /// consumed by #772 rollback and the UI deployment list).
    pub fn append_deployment(&self, id: Uuid, record: &serde_json::Value) -> Result<(), String> {
        let mut conn = self.conn()?;
        diesel::sql_query(
            "UPDATE vibe_projects
             SET payload = jsonb_set(
                   payload,
                   '{deployments}',
                   COALESCE(payload->'deployments', '[]'::jsonb) || $2::jsonb
                 ),
                 updated_at = NOW()
             WHERE id = $1",
        )
        .bind::<diesel::sql_types::Uuid, _>(id)
        .bind::<diesel::sql_types::Jsonb, _>(record)
        .execute(&mut conn)
        .map_err(|e| format!("append deployment: {e}"))?;
        Ok(())
    }

    /// #1160 — Records the desktop launch surface of a published project
    /// (`{enabled, kind: app|widget, at, env}`) in `vibe_projects.payload`
    /// under the `launcher` key. The desktop launcher reads it to auto-pin
    /// apps published with `launcher: true`.
    pub fn set_launcher(&self, id: Uuid, spec: &serde_json::Value) -> Result<(), String> {
        let mut conn = self.conn()?;
        diesel::sql_query(
            "UPDATE vibe_projects
             SET payload = jsonb_set(payload, '{launcher}', $2::jsonb),
                 updated_at = NOW()
             WHERE id = $1",
        )
        .bind::<diesel::sql_types::Uuid, _>(id)
        .bind::<diesel::sql_types::Jsonb, _>(spec)
        .execute(&mut conn)
        .map_err(|e| format!("set launcher: {e}"))?;
        Ok(())
    }

    /// #772 — Deployment history for a project, newest first, optionally
    /// filtered by environment. Reads the `deployments` jsonb array that
    /// `append_deployment` accumulates in `vibe_projects.payload`.
    pub fn list_deployments(
        &self,
        id: Uuid,
        env: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let project = self.get(id)?.ok_or_else(|| format!("project {id} not found"))?;
        let arr = project
            .payload
            .get("deployments")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut rows: Vec<serde_json::Value> = arr
            .into_iter()
            .filter(|d| {
                env.map(|e| d.get("env").and_then(|v| v.as_str()) == Some(e))
                    .unwrap_or(true)
            })
            .collect();
        rows.sort_by(|a, b| deploy_at(b).cmp(deploy_at(a)));
        Ok(rows)
    }

    pub fn update(&self, id: Uuid, req: &UpdateProjectRequest) -> Result<bool, String> {
        let mut conn = self.conn()?;
        let mut assignments: Vec<String> = Vec::new();
        let mut binds: Vec<String> = Vec::new();
        let mut payload_bind: Option<serde_json::Value> = None;

        if let Some(ref name) = req.name {
            binds.push(name.clone());
            assignments.push(format!("name = ${}", binds.len()));
        }
        if let Some(ref pt) = req.project_type {
            // #1372 — same strictness on update: an invalid kind must not
            // flip an existing project onto the VM path.
            let kind = ProjectKind::parse_strict(pt)?;
            binds.push(kind.as_str().to_string());
            assignments.push(format!("project_type = ${}", binds.len()));
        }
        if let Some(ref repo) = req.repository {
            binds.push(repo.clone());
            assignments.push(format!("repository = ${}", binds.len()));
        }
        if let Some(ref fw) = req.framework {
            binds.push(fw.clone());
            assignments.push(format!("framework = ${}", binds.len()));
        }
        if let Some(ref cd) = req.custom_domain {
            binds.push(cd.clone());
            assignments.push(format!("custom_domain = ${}", binds.len()));
        }
        if let Some(ref sc) = req.source_control {
            // #1503 — github (external import) stays opt-in; everything else
            // defaults to git (Forgejo-backed) — native is legacy-only.
            let sc = match sc.as_str() {
                "git" | "github" => sc.clone(),
                _ => "git".to_string(),
            };
            binds.push(sc);
            assignments.push(format!("source_control = ${}", binds.len()));
        }
        if let Some(ref env) = req.environment {
            binds.push(env.clone());
            assignments.push(format!("environment = ${}", binds.len()));
        }
        if let Some(ref st) = req.status {
            binds.push(st.clone());
            assignments.push(format!("status = ${}", binds.len()));
        }
        if let Some(ref p) = req.payload {
            payload_bind = Some(p.clone());
            assignments.push(format!("payload = ${}", binds.len() + 1));
        }

        if assignments.is_empty() {
            return Ok(false);
        }
        assignments.push("updated_at = NOW()".to_string());

        let mut sql = String::from("UPDATE vibe_projects SET ");
        sql.push_str(&assignments.join(", "));
        sql.push_str(&format!(" WHERE id = ${}", binds.len() + (payload_bind.is_some() as usize) + 1));

        let mut query_builder = diesel::sql_query(sql).into_boxed::<diesel::pg::Pg>();
        for b in &binds {
            query_builder = query_builder.bind::<diesel::sql_types::Text, _>(b.clone());
        }
        if let Some(p) = payload_bind {
            query_builder = query_builder.bind::<diesel::sql_types::Jsonb, _>(p);
        }
        query_builder = query_builder.bind::<diesel::sql_types::Uuid, _>(id);

        let affected = query_builder
            .execute(&mut conn)
            .map_err(|e| format!("update project: {e}"))?;
        Ok(affected > 0)
    }

    pub fn delete(&self, id: Uuid) -> Result<bool, String> {
        let mut conn = self.conn()?;
        let affected = diesel::sql_query("DELETE FROM vibe_projects WHERE id = $1")
            .bind::<diesel::sql_types::Uuid, _>(id)
            .execute(&mut conn)
            .map_err(|e| format!("delete project: {e}"))?;
        Ok(affected > 0)
    }
}

/// Extract the deployment timestamp (`at` field) for history sorting.
pub(crate) fn deploy_at(v: &serde_json::Value) -> &str {
    v.get("at").and_then(|t| t.as_str()).unwrap_or("")
}

pub type ProjectRegistryRef = Arc<ProjectRegistry>;
