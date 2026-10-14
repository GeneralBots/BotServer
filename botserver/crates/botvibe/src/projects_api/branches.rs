//! `projects_api::branches` — split per #1443 (AGENTS.md 450-line rule).

use super::*;

#[derive(Debug, Serialize)]
pub struct ProjectResponse {
    pub success: bool,
    pub project: Option<Project>,
    pub projects: Option<Vec<Project>>,
    pub error: Option<String>,
    /// Machine-readable error class so the UI can branch on specific
    /// protections (e.g. `default_bot_project_protected`, #1440).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Resolve the caller's active branch for an org so project creation and
/// metering run against the real tenant scope instead of the nil branch.
pub(crate) fn resolve_org_branch(registry: &ProjectRegistryRef, org_id: Uuid) -> Option<Uuid> {
    if org_id.is_nil() {
        return None;
    }
    match registry.conn() {
        Ok(mut conn) => {
            #[derive(diesel::QueryableByName)]
            struct BranchRow {
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                id: Uuid,
            }
            diesel::sql_query(
                "SELECT id FROM branches WHERE org_id = $1 AND is_active = true \
                 ORDER BY created_at ASC LIMIT 1",
            )
            .bind::<diesel::sql_types::Uuid, _>(org_id)
            .get_result::<BranchRow>(&mut conn)
            .optional()
            .ok()
            .flatten()
            .map(|r| r.id)
        }
        Err(e) => {
            log::error!("Vibe: resolve branch for org {org_id} failed: {e}");
            None
        }
    }
}

pub(crate) async fn delete_project(
    Extension(registry): Extension<ProjectRegistryRef>,
    Extension(rbac): Extension<ProjectRbac>,
    Extension(lifecycle): Extension<Arc<VmLifecycle>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
    Query(query): Query<DeleteProjectQuery>,
) -> ApiResult {
    // #1440 — platform/service admins manage any project; the branch-default
    // project is auto-created by bootstrap and may have no explicit Owner row
    // for the calling admin (the signup-grant covers the org owner only).
    let is_admin = user.roles.iter().any(|r| matches!(r, Role::Admin | Role::SuperAdmin | Role::Service));
    if !is_admin {
        match rbac.require_role(user.user_id, id, ProjectRole::Owner) {
            Ok(_) => {}
            Err(e) => return forbidden(e),
        }
    }
    // Fetch the project first: asset cleanup needs its name/workspace key.
    let project = match registry.get(id) {
        Ok(Some(p)) => p,
        Ok(None) => return err_response(format!("project {id} not found")),
        Err(e) => return err_response(e),
    };
    // #1440 — explicit protection instead of a silently broken delete: a
    // bot-kind project whose slug equals the branch's default bot owns the
    // branch bot's PROD identity (an ADOPTED bots row that outlives the
    // project). Deleting it would strand the adopted bot (chat, channels,
    // `bot_{branch}_{bot}` database per #1386) with no project managing it.
    // Require a ?force=true ack from an Owner/admin; the UI surfaces this.
    if project.project_type == "bot" {
        let prod_slug = crate::bootstrap::bot_slug(&project.name);
        let adopted: Result<Option<OneRow>, _> = diesel::sql_query(
            "SELECT 1 AS one FROM bots \
             WHERE slug = $1 AND branch_id = $2 AND (origin IS DISTINCT FROM 'vibe') LIMIT 1",
        )
        .bind::<diesel::sql_types::Text, _>(&prod_slug)
        .bind::<diesel::sql_types::Uuid, _>(project.branch_id)
        .get_result::<OneRow>(&mut match registry.pool().get() {
            Ok(c) => c,
            Err(e) => return err_response(format!("db pool: {e}")),
        })
        .optional();
        // A lookup failure must not silently unblock the protected delete:
        // treat DB errors as "adopted present" so only ?force=true proceeds.
        let is_adopted = match adopted {
            Ok(Some(row)) => row.one == 1,
            Ok(None) => false,
            Err(_) => true,
        };
        if is_adopted && !query.force {
            return (
                StatusCode::CONFLICT,
                Json(ProjectResponse {
                    success: false,
                    project: None,
                    projects: None,
                    error: Some(
                        "This is the branch's default bot project. Its PROD bot identity is \
                         adopted (not owned), so deletion would strand the branch bot. \
                         Repeat the request with ?force=true to delete anyway."
                            .to_string(),
                    ),
                    code: Some("default_bot_project_protected".to_string()),
                }),
            );
        }
    }
    // Shared asset cleanup: Incus VMs (rows + containers), published proxy
    // site (payload + route + systemd unit), on-disk workspace directory —
    // the workspace removal closes the disk leak (workspaces could hold
    // node_modules/venvs forever after the project row was gone).
    for e in crate::eviction::delete_project_assets(&project, &lifecycle).await {
        log::warn!("Vibe: asset cleanup for project {id}: {e}");
    }
    // Every project owns a Forgejo repo: git-mode projects by definition, and
    // native-mode ones that were previously git-mode (or were seeded from a
    // template) still have one. Gating on `source_control == "git"` left those
    // repos behind — an `e2e-apps-run` project deletion kept `eb4ca084/
    // e2e-apps-run` in ALM forever. `delete_repository` maps 404 to success,
    // so attempting it for every project is idempotent and safe.
    {
        let (alm_base, alm_token, _org) = botcoresecrets::alm_config();
        if !alm_base.is_empty() && !alm_token.is_empty() {
            let forgejo_org = crate::vm_lifecycle::VmLifecycle::alm_org(project.branch_id);
            let forgejo_repo = crate::vm_lifecycle::VmLifecycle::alm_repo(&project.name);
            let client = botdeployment::ForgejoClient::new(alm_base, alm_token);
            match client
                .delete_repository(&forgejo_org, &forgejo_repo)
                .await
            {
                Ok(_) => log::info!(
                    "Vibe {}: deleted Forgejo repo {forgejo_org}/{forgejo_repo}",
                    project.source_control
                ),
                Err(e) => log::warn!(
                    "Vibe {}: delete Forgejo repo {forgejo_org}/{forgejo_repo} failed: {e}",
                    project.source_control
                ),
            }
        }
    }
    // #1386/#1504 follow-up — a bot-kind project owns its `bots` rows (the
    // PROD identity and the `-test` twin, both created by
    // bootstrap::ensure_bot_rows with origin='vibe'); deleting the project
    // removes them so the slugs/chat identities do not outlive the project.
    // An ADOPTED PROD row (origin <> 'vibe', e.g. the branch default bot)
    // is never deleted — the project took it over, not owns it. Non-fatal.
    if project.project_type == "bot" {
        if let Ok(mut conn) = registry.pool().get() {
            let prod_slug = crate::bootstrap::bot_slug(&project.name);
            let test_slug = format!("{prod_slug}-test");
            for slug in [prod_slug, test_slug] {
                match diesel::sql_query(
                    "DELETE FROM bots WHERE slug = $1 AND origin = 'vibe' AND branch_id = $2",
                )
                .bind::<diesel::sql_types::Text, _>(&slug)
                .bind::<diesel::sql_types::Uuid, _>(project.branch_id)
                .execute(&mut conn)
                {
                    Ok(n) if n > 0 => log::info!(
                        "vibe bot row deleted: slug={slug} project={id}"
                    ),
                    Ok(_) => {}
                    Err(e) => log::error!(
                        "vibe bot row delete failed for project {id} (slug {slug}): {e}"
                    ),
                }
            }
        }
    }
    match registry.delete(id) {
        Ok(true) => deleted(),
        Ok(false) => err_response(format!("project {id} not found")),
        Err(e) => err_response(e),
    }
}

pub(crate) async fn list_projects(
    Extension(registry): Extension<ProjectRegistryRef>,
    Extension(user): Extension<AuthenticatedUser>,
    Query(query): Query<ListProjectsQuery>,
) -> ApiResult {
    // Gate on roles, not on `user_id.is_nil()`: an internal service caller
    // (`AuthenticatedUser::service`) carries `user_id = Uuid::nil()` but the
    // `Service` role, so the old check rejected it with 403 while `delete_project`
    // — which gates on the same roles — accepted it. A service token could
    // delete a project but not list one, forcing operators into raw SQL.
    let is_privileged = user
        .roles
        .iter()
        .any(|r| matches!(r, Role::Admin | Role::SuperAdmin | Role::Service));
    if !is_privileged && user.user_id.is_nil() {
        return forbidden("forbidden: anonymous users cannot list projects".into());
    }
    // #1267 — resolve the caller's real org branch when the query does not
    // carry one, mirroring create_project. Otherwise a project created in the
    // org branch (see create) is invisible to the list, which would default
    // to the nil branch and return an empty/stale set (#931 scope mismatch).
    // A privileged caller with no org has no branch to resolve, so it lists
    // across every branch instead of silently matching the empty nil branch.
    let query = if query.all_branches {
        query
    } else if query.branch_id.is_none() && is_privileged {
        ListProjectsQuery {
            all_branches: true,
            ..query
        }
    } else if query.branch_id.is_none() {
        let org_id = user.organization_id.unwrap_or_else(Uuid::nil);
        ListProjectsQuery {
            branch_id: resolve_org_branch(&registry, org_id).or(query.branch_id),
            ..query
        }
    } else {
        query
    };
    match registry.list(&query) {
        Ok(list) => ok_projects(list),
        Err(e) => err_response(e),
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct BranchInfo {
    pub(crate) name: String,
    pub(crate) current: bool,
}

pub(crate) async fn list_project_branches(
    Extension(registry): Extension<ProjectRegistryRef>,
    Extension(rbac): Extension<ProjectRbac>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(project_id): Path<Uuid>,
) -> (StatusCode, Json<serde_json::Value>) {
    if let Err(e) = rbac.require_role(user.user_id, project_id, ProjectRole::Viewer) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({ "success": false, "error": e })));
    }
    let project = match registry.get(project_id) {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "success": false, "error": "project not found" })));
        }
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "success": false, "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botvibe::projects_api::branches", None)}))),
    };
    let key = workspace_key(&project);
    let cwd = match harness::ensure_workspace(&key) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "success": false, "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botvibe::projects_api::branches", None)}))),
    };
    let branches = harness::cmd::run(
        "git",
        &["branch".to_string(), "--format=%(refname:short)".to_string()],
        &cwd,
        30,
    );
    let current = harness::cmd::run(
        "git",
        &["rev-parse".to_string(), "--abbrev-ref".to_string(), "HEAD".to_string()],
        &cwd,
        15,
    )
    .ok()
    .filter(|o| o.exit_code == Some(0))
    .map(|o| o.stdout.trim().to_string())
    .unwrap_or_default();
    let mut out: Vec<BranchInfo> = Vec::new();
    if let Ok(b) = branches {
        if b.exit_code == Some(0) {
            for line in b.stdout.lines() {
                let name = line.trim();
                if name.is_empty() {
                    continue;
                }
                out.push(BranchInfo {
                    name: name.to_string(),
                    current: name == current,
                });
            }
        }
    }
    if out.is_empty() {
        out.push(BranchInfo {
            name: if current.is_empty() { "main".to_string() } else { current.clone() },
            current: true,
        });
    }
    (
        StatusCode::OK,
        Json(serde_json::json!({ "success": true, "branches": out, "current": current })),
    )
}

pub(crate) async fn switch_project_branch(
    Extension(registry): Extension<ProjectRegistryRef>,
    Extension(rbac): Extension<ProjectRbac>,
    Extension(user): Extension<AuthenticatedUser>,
    Path((project_id, name)): Path<(Uuid, String)>,
) -> (StatusCode, Json<serde_json::Value>) {
    if let Err(e) = rbac.require_role(user.user_id, project_id, ProjectRole::Developer) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({ "success": false, "error": e })));
    }
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "success": false, "error": "invalid branch name" })));
    }
    let project = match registry.get(project_id) {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "success": false, "error": "project not found" })));
        }
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "success": false, "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botvibe::projects_api::branches", None)}))),
    };
    let key = workspace_key(&project);
    let cwd = match harness::ensure_workspace(&key) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "success": false, "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botvibe::projects_api::branches", None)}))),
    };
    let exists = harness::cmd::run(
        "git",
        &[
            "rev-parse".to_string(),
            "--verify".to_string(),
            format!("refs/heads/{name}"),
        ],
        &cwd,
        15,
    )
    .map(|o| o.exit_code == Some(0))
    .unwrap_or(false);
    let cmd: Vec<String> = if exists {
        vec!["checkout".to_string(), name.clone()]
    } else {
        vec!["checkout".to_string(), "-B".to_string(), name.clone()]
    };
    match harness::cmd::run("git", &cmd, &cwd, 60) {
        Ok(out) if out.exit_code == Some(0) => (
            StatusCode::OK,
            Json(serde_json::json!({ "success": true, "branch": name, "output": out.stdout.trim() })),
        ),
        Ok(out) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "success": false, "error": format!("git checkout failed: {}", out.stderr.trim()) })),
        ),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "success": false, "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e.to_string(), "botvibe::projects_api::branches", None) }))),
    }
}
