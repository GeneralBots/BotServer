//! `projects_api::workspace` — split per #1443 (AGENTS.md 450-line rule).

use super::*;

pub(crate) async fn create_project(
    Extension(registry): Extension<ProjectRegistryRef>,
    Extension(rbac): Extension<ProjectRbac>,
    Extension(metering): Extension<Arc<VMetering>>,
    Extension(lifecycle): Extension<Arc<VmLifecycle>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(req): Json<CreateProjectRequest>,
) -> ApiResult {
    if user.user_id.is_nil() {
        return forbidden("forbidden: anonymous users cannot create projects".into());
    }
    if req.name.trim().is_empty() {
        return err_response("project name must not be empty".into());
    }
    // #918 — tenant scope comes from the authenticated principal, never from
    // untrusted request fields; a caller cannot create a project under another
    // organization.
    let mut req = req;
    req.org_id = Some(user.organization_id.unwrap_or_else(Uuid::nil));
    let org_id = req.org_id.unwrap_or_else(Uuid::nil);
    // #1267 — resolve the caller's real org branch when the request does not
    // carry one, so metering sees the org's actual plan (a nil branch always
    // resolves to the Free plan and wrongly blocks custom projects even for
    // private-cloud subscribers) and the project lands in the right tenant
    // scope instead of the nil branch.
    let branch_id = match req.branch_id {
        Some(bid) => bid,
        None => resolve_org_branch(&registry, org_id).unwrap_or_else(Uuid::nil),
    };
    req.branch_id = Some(branch_id);
    if let Err(e) = metering.enforce_project_creation(
        org_id,
        branch_id,
        req.project_type.as_deref().unwrap_or("bot"),
    ) {
        return forbidden(e);
    }
    // #1386 — a bot-kind project cannot take a `-dev`-suffixed name: that
    // suffix is reserved for the Vibe dev-twin bot (and its
    // `bot_..._dev` per-environment database).
    if req.project_type.as_deref().unwrap_or("bot") == "bot"
        && botcore::bot_database::BotDatabaseManager::is_reserved_dev_bot_name(req.name.trim())
    {
        return forbidden(
            "project names ending in '-dev' are reserved for dev-twin bots".into(),
        );
    }
    // Disk-guard eviction: a branch keeps at most
    // VIBE_MAX_PROJECTS_PER_KIND (default 2) projects of each kind. Creating
    // beyond the cap evicts the OLDEST same-kind project with full asset
    // cleanup (VMs, published site, workspace dir) so workspaces and stopped
    // VM containers cannot grow without bound and exhaust the host disk.
    match crate::eviction::evict_oldest_if_needed(
        &registry,
        &lifecycle,
        branch_id,
        req.project_type.as_deref().unwrap_or("bot"),
    )
    .await
    {
        Ok(evicted) if !evicted.is_empty() => {
            log::info!("Vibe: eviction completed for branch {branch_id}: {evicted:?}");
        }
        Ok(_) => {}
        Err(e) => return err_response(format!("project eviction failed: {e}")),
    }
    match registry.create(&req) {
        Ok(p) => {
            // #1386 — a `bot`-kind Vibe project IS a real bot: ensure its
            // `bots` row (branch-scoped, public for the WS gateway) so the
            // project surfaces in the desktop app launcher (`bot-{slug}`
            // tile, src/apps/mod.rs), at `/chat/{slug}` and every other
            // bot-keyed subsystem. Non-fatal: failure logs and the project
            // continues to work in Vibe itself.
            // #1386 — every project kind (bot, website, apps) owns its
            // database pair from creation: `app_{branch}_{name}` and the
            // `_dev` twin. One project, one database, both environments —
            // the Database pane, the dev VM and the bot itself must all see
            // the same database. Best-effort: a failure is reported by the
            // pane, which creates a missing database on demand.
            for env in ["production", "test"] {
                if let Err(e) = crate::project_db::ensure_project_database(
                    registry.pool(),
                    p.branch_id,
                    &p.name,
                    env,
                ) {
                    log::warn!(
                        "Vibe create: {env} database for project {} unavailable: {e}",
                        p.name
                    );
                }
            }
            if p.project_type == "bot" {
                // #1504 — bot projects own BOTH bot identities (PROD + TEST
                // twin); the bootstrap helper also records the adopted bots
                // row into payload.bot_id for the default-bot flow (#1500).
                crate::bootstrap::ensure_bot_rows(
                    registry.pool(),
                    &crate::bootstrap::BotRowRef::from_project(&p),
                    req.description.as_deref(),
                );
            }
            // Grant ownership BEFORE seeding: a project the caller cannot
            // administer must not be visible in their list (#931).
            if let Err(e) = rbac.set_user_role(p.id, user.user_id, ProjectRole::Owner) {
                log::error!("grant owner on project {} failed: {e}", p.id);
                if let Err(de) = registry.delete(p.id) {
                    log::error!("compensating delete for project {} failed: {de}", p.id);
                }
                return err_response(format!("grant project ownership failed: {e}"));
            }
            // Vibe owns its starter content: seed the built-in template
            // (calculator for calculator-style projects, README starter
            // otherwise) so the workspace is never empty and nothing depends
            // on a pre-seeded external tree. Never clobbers agent output.
            // A failed seed must not leave an orphan project row whose
            // workspace is permanently empty (#931).
            let key = workspace_key(&p);
            // #1271 — github-mode clones the caller's repository into the
            // workspace; seeding the built-in template first would make the
            // clone target non-empty and fail. Clone replaces the seed.
            let wants_seed = p.source_control != "github";
            if wants_seed {
                log::info!(
                    "Vibe create: seeding project {} (key={key}, desc={})",
                    p.name,
                    req.description.as_ref().map(|d| d.len()).unwrap_or(0)
                );
                // #1312 — LLM-first starter: the description ("what do you
                // want to build?") scaffolds the workspace via the LLM; the
                // built-in template is only the offline fallback.
                if let Err(e) = crate::scaffold::scaffold_project_workspace(
                    &key,
                    &p.name,
                    &p.project_type,
                    p.framework.as_deref(),
                    req.description.as_deref(),
                )
                .await
                {
                    log::error!("seed workspace for project {} failed: {e}", p.id);
                    if let Err(de) = registry.delete(p.id) {
                        log::error!("compensating delete for project {} failed: {de}", p.id);
                    }
                    return err_response(format!("seed project workspace failed: {e}"));
                }
            }
            // #1271 — git-mode projects get a real Forgejo repo + origin
            // remote so Deploy can snapshot per-deploy branches and promote
            // dev→prod by runtime. A git wiring failure is logged, not fatal:
            // the project still works in native mode until ALM is configured.
            // github-mode projects clone the caller's external repository into
            // the workspace instead (payload.clone_url).
            if p.source_control == "github" {
                if let Err(e) = crate::git_mode::ensure_github_clone(&p).await {
                    log::error!("github-mode wiring for project {} failed: {e}", p.id);
                }
            } else if p.source_control == "git" {
                if let Err(e) = crate::git_mode::ensure_git_repo(&p).await {
                    log::error!("git-mode wiring for project {} failed: {e}", p.id);
                }
            }
            ok_project(p)
        }
        Err(e) => err_response(e),
    }
}

pub(crate) async fn update_project(
    Extension(registry): Extension<ProjectRegistryRef>,
    Extension(rbac): Extension<ProjectRbac>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProjectRequest>,
) -> ApiResult {
    match rbac.require_role(user.user_id, id, ProjectRole::Developer) {
        Ok(_) => {}
        Err(e) => return forbidden(e),
    }
    match registry.update(id, &req) {
        Ok(true) => match registry.get(id) {
            Ok(Some(p)) => ok_project(p),
            Ok(None) => err_response(format!("project {id} not found")),
            Err(e) => err_response(e),
        },
        Ok(false) => err_response(format!("project {id} not found or no changes")),
        Err(e) => err_response(e),
    }
}

pub(crate) async fn get_project(
    Extension(registry): Extension<ProjectRegistryRef>,
    Extension(rbac): Extension<ProjectRbac>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> ApiResult {
    match rbac.require_role(user.user_id, id, ProjectRole::Viewer) {
        Ok(_) => {}
        Err(e) => return forbidden(e),
    }
    match registry.get(id) {
        Ok(Some(p)) => ok_project(p),
        Ok(None) => err_response(format!("project {id} not found")),
        Err(e) => err_response(e),
    }
}

/// #1504 — explicit TEST recompile for a bot project (Run alternative for
/// callers that already have the VM running). Dispatches to the main
/// binary's git-monitor hook.
pub(crate) async fn bot_run_test(
    Extension(_registry): Extension<ProjectRegistryRef>,
    Extension(rbac): Extension<ProjectRbac>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(project_id): Path<Uuid>,
) -> (StatusCode, Json<serde_json::Value>) {
    if let Err(e) = rbac.require_role(user.user_id, project_id, ProjectRole::Developer) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({ "success": false, "error": e })));
    }
    match botcoresecrets::hooks::call_bot_project_ops("run-test", project_id) {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({ "success": true, "bot_env": "test" })),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "success": false, "error": e })),
        ),
    }
}

/// #1504 — PROD promotion for a bot project (Deploy pipeline calls this).
/// Dispatches to the main binary's git-monitor hook.
pub(crate) async fn bot_deploy_prod(
    Extension(_registry): Extension<ProjectRegistryRef>,
    Extension(rbac): Extension<ProjectRbac>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(project_id): Path<Uuid>,
) -> (StatusCode, Json<serde_json::Value>) {
    if let Err(e) = rbac.require_role(user.user_id, project_id, ProjectRole::Developer) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({ "success": false, "error": e })));
    }
    match botcoresecrets::hooks::call_bot_project_ops("deploy-prod", project_id) {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({ "success": true, "bot_env": "production" })),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "success": false, "error": e })),
        ),
    }
}

pub fn projects_router(
    registry: ProjectRegistryRef,
    rbac: ProjectRbac,
    metering: Arc<VMetering>,
    lifecycle: Arc<VmLifecycle>,
) -> axum::Router {
    use axum::routing::{any, delete, get, post, put};
    axum::Router::new()
        .route("/api/vibe/projects", post(create_project))
        .route("/api/vibe/projects", get(list_projects))
        .route("/api/vibe/projects/:project_id", get(get_project))
        .route("/api/vibe/projects/:project_id", put(update_project))
        .route("/api/vibe/projects/:project_id", delete(delete_project))
        .route("/api/vibe/projects/:project_id/files", get(list_project_files))
        .route("/api/vibe/projects/:project_id/files/content", get(read_project_file))
        .route("/api/vibe/projects/:project_id/files", post(write_project_file))
        .route("/api/vibe/projects/:project_id/export", get(export_project))
        .route("/api/vibe/projects/:project_id/git/pr", post(create_project_pr))
        // #1192 — run the project's own custom app: stream the workspace
        // files (the LLM-generated source) so Play/Preview opens the real app
        // instead of a bundled template. Same per-project RBAC as the other
        // workspace endpoints; `?token=` is honored like the WS routes so the
        // embedded Browser iframe can load it directly.
        .route("/api/vibe/projects/:project_id/serve/*path", get(serve_project_file))
        // #1271 — Run actually starts the app as a process in the dev VM
        // (node visible in the project terminal's `ps`), exposed through a
        // host proxy device; the browser opens the returned URL instead of a
        // static workspace stream.
        .route("/api/vibe/projects/:project_id/run", post(run_project_app))
        // #1504 — two-environment bot ops: Run recompiles the `{bot}-test`
        // twin from the workspace; Deploy promotes the TEST release into the
        // `{bot}` PROD layout. Both dispatch through the main binary's hook
        // (git monitor) because the compile pipeline lives there.
        .route(
            "/api/vibe/projects/:project_id/bot/run-test",
            post(bot_run_test),
        )
        .route(
            "/api/vibe/projects/:project_id/bot/deploy-prod",
            post(bot_deploy_prod),
        )
        // #1271 — same-origin preview of the running dev VM (server-side
        // fetch of the host proxy port, see `preview_vm_app`). Any method is
        // forwarded (a dev app with POST/DELETE endpoints is usable in the
        // embedded Browser); a GET-only route answered 405.
        .route("/api/vibe/projects/:project_id/vm-preview", any(preview_vm_app))
        // Branch combo over the real workspace repo (the old /api/git/*
        // endpoints resolve any non-/tmp repo to a fixed stub repo, so the
        // combo never showed the project's branches).
        .route("/api/vibe/projects/:project_id/branches", get(list_project_branches))
        .route(
            "/api/vibe/projects/:project_id/branches/:name",
            post(switch_project_branch),
        )
        // Project Properties: full run history with per-run and rolled-up
        // token usage (input/output/total) for the selected project.
        .route(
            "/api/vibe/projects/:project_id/history",
            get(project_run_history),
        )
        .layer(Extension(lifecycle))
        .layer(Extension(registry))
        .layer(Extension(rbac))
        .layer(Extension(metering))
}

// ── Workspace file browser (load a project's actual agent output) ────────────
// The code editor used to read `/api/editor/files` (the bot's global editor
// workspace), which is unrelated to the per-project Vibe workspace. These
// endpoints expose the real `VIBE_WORKSPACE_ROOT/{slug}/` files so selecting
// a project in the sidebar loads its source instead of nothing.
#[derive(Debug, Deserialize)]
pub struct WorkspaceFileQuery {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct WriteWorkspaceFileRequest {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceFilesResponse {
    pub success: bool,
    pub project_id: Option<Uuid>,
    pub workspace: Option<String>,
    pub files: Option<Vec<String>>,
    pub path: Option<String>,
    pub content: Option<String>,
    pub bytes: Option<usize>,
    pub error: Option<String>,
}

pub(crate) fn workspace_key(project: &Project) -> String {
    VmLifecycle::alm_repo(&project.name)
}

pub(crate) fn ws_ok(project_id: Uuid, key: String) -> WorkspaceFilesResponse {
    WorkspaceFilesResponse {
        success: true,
        project_id: Some(project_id),
        workspace: Some(key),
        files: None,
        path: None,
        content: None,
        bytes: None,
        error: None,
    }
}
