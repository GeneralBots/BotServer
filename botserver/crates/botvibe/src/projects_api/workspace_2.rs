//! `projects_api::workspace_2` — split per #1443 (AGENTS.md 450-line rule).

use super::*;

pub(crate) fn ws_err(project_id: Option<Uuid>, key: Option<String>, msg: String) -> (StatusCode, Json<WorkspaceFilesResponse>) {
    log::error!("Vibe workspace files API error: {msg}");
    (
        StatusCode::OK,
        Json(WorkspaceFilesResponse {
            success: false,
            project_id,
            workspace: key,
            files: None,
            path: None,
            content: None,
            bytes: None,
            error: Some(msg),
        }),
    )
}

pub(crate) fn ws_forbidden(msg: String) -> (StatusCode, Json<WorkspaceFilesResponse>) {
    log::warn!("Vibe workspace files API forbidden: {msg}");
    (
        StatusCode::FORBIDDEN,
        Json(WorkspaceFilesResponse {
            success: false,
            project_id: None,
            workspace: None,
            files: None,
            path: None,
            content: None,
            bytes: None,
            error: Some(msg),
        }),
    )
}

pub(crate) async fn list_project_files(
    Extension(registry): Extension<ProjectRegistryRef>,
    Extension(rbac): Extension<ProjectRbac>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> (StatusCode, Json<WorkspaceFilesResponse>) {
    if let Err(e) = rbac.require_role(user.user_id, id, ProjectRole::Viewer) {
        return ws_forbidden(e);
    }
    let project = match registry.get(id) {
        Ok(Some(p)) => p,
        Ok(None) => return ws_err(Some(id), None, format!("project {id} not found")),
        Err(e) => return ws_err(Some(id), None, e),
    };
    let key = workspace_key(&project);
    match harness::list_rel(&key, "", 0) {
        Ok(mut files) => {
            files.sort();
            let mut resp = ws_ok(id, key);
            resp.files = Some(files);
            (StatusCode::OK, Json(resp))
        }
        Err(e) => ws_err(Some(id), Some(key), e),
    }
}

pub(crate) async fn read_project_file(
    Extension(registry): Extension<ProjectRegistryRef>,
    Extension(rbac): Extension<ProjectRbac>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
    Query(query): Query<WorkspaceFileQuery>,
) -> (StatusCode, Json<WorkspaceFilesResponse>) {
    if let Err(e) = rbac.require_role(user.user_id, id, ProjectRole::Viewer) {
        return ws_forbidden(e);
    }
    let project = match registry.get(id) {
        Ok(Some(p)) => p,
        Ok(None) => return ws_err(Some(id), None, format!("project {id} not found")),
        Err(e) => return ws_err(Some(id), None, e),
    };
    let key = workspace_key(&project);
    match harness::read_rel_file(&key, &query.path, 4 * 1024 * 1024) {
        Ok(bytes) => {
            let content = String::from_utf8_lossy(&bytes).into_owned();
            let mut resp = ws_ok(id, key);
            resp.path = Some(query.path.clone());
            resp.content = Some(content);
            resp.bytes = Some(bytes.len());
            (StatusCode::OK, Json(resp))
        }
        Err(e) => {
            let mut resp = ws_err(Some(id), Some(key), e);
            resp.1 .0.path = Some(query.path.clone());
            resp
        }
    }
}

pub(crate) async fn export_project(
    Extension(registry): Extension<ProjectRegistryRef>,
    Extension(rbac): Extension<ProjectRbac>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> (StatusCode, Json<ExportResponse>) {
    if let Err(e) = rbac.require_role(user.user_id, id, ProjectRole::Viewer) {
        return (
            StatusCode::FORBIDDEN,
            Json(ExportResponse {
                success: false,
                project_id: Some(id),
                name: None,
                files: None,
                error: Some(e),
            }),
        );
    }
    let project = match registry.get(id) {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ExportResponse {
                    success: false,
                    project_id: Some(id),
                    name: None,
                    files: None,
                    error: Some(format!("project {id} not found")),
                }),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ExportResponse {
                    success: false,
                    project_id: Some(id),
                    name: None,
                    files: None,
                    error: Some(e),
                }),
            );
        }
    };
    let key = workspace_key(&project);
    let paths = match harness::list_rel(&key, "", 0) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::OK,
                Json(ExportResponse {
                    success: false,
                    project_id: Some(id),
                    name: Some(project.name.clone()),
                    files: None,
                    error: Some(e),
                }),
            );
        }
    };
    let mut files: Vec<ExportFile> = Vec::new();
    for path in paths {
        if let Ok(bytes) = harness::read_rel_file(&key, &path, 4 * 1024 * 1024) {
            use base64::Engine as _;
            files.push(ExportFile {
                path,
                bytes: bytes.len(),
                content_base64: base64::engine::general_purpose::STANDARD.encode(&bytes),
            });
        }
    }
    (
        StatusCode::OK,
        Json(ExportResponse {
            success: true,
            project_id: Some(id),
            name: Some(project.name.clone()),
            files: Some(files),
            error: None,
        }),
    )
}

pub(crate) async fn create_project_pr(
    Extension(registry): Extension<ProjectRegistryRef>,
    Extension(rbac): Extension<ProjectRbac>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
    Json(req): Json<CreatePrRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    if let Err(e) = rbac.require_role(user.user_id, id, ProjectRole::Developer) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({ "success": false, "error": e })));
    }
    let project = match registry.get(id) {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "success": false, "error": "project not found" })));
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "success": false, "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botvibe::projects_api::workspace_2", None)})));
        }
    };
    let result = crate::gitflow::create_pull_request(
        &project.name,
        &req.title,
        &req.head,
        &req.base,
        &req.body,
    )
    .await;
    if result.success {
        (StatusCode::OK, Json(serde_json::json!({ "success": true, "data": result.data })))
    } else {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "success": false, "error": result.error })),
        )
    }
}

// ── #1271: run the project app as a real process in the dev VM ────────────
// Run pushes the workspace files into the dev container, starts node (or a
// generated static server) as a systemd service, exposes it through a host
// proxy device and returns the URL. The app process is then visible in the
// project terminal's `ps` — previously the browser streamed workspace files
// with nothing actually running on the VM.
#[derive(Debug, Deserialize)]
pub struct RunProjectQuery {
    /// Host port for the proxy device; stable per project when omitted.
    pub port: Option<u16>,
}

pub(crate) async fn run_project_app(
    Extension(registry): Extension<ProjectRegistryRef>,
    Extension(rbac): Extension<ProjectRbac>,
    Extension(lifecycle): Extension<Arc<VmLifecycle>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(project_id): Path<Uuid>,
    Query(query): Query<RunProjectQuery>,
) -> (StatusCode, Json<serde_json::Value>) {
    if let Err(e) = rbac.require_role(user.user_id, project_id, ProjectRole::Viewer) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({ "success": false, "error": e })));
    }
    let project = match registry.get(project_id) {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "success": false, "error": "project not found" })));
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "success": false, "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botvibe::projects_api::workspace_2", None)})));
        }
    };
    let files = match crate::publish::collect_workspace_files(&project) {
        Ok(f) => f,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "success": false, "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botvibe::projects_api::workspace_2", None)}))),
    };
    // #1271 — a project with an empty workspace must still serve an app when
    // Run is clicked (automatic project creation skips the explicit seeding in
    // `create_project`). Seed the starter (or calculator) template on the fly
    // so the Browser never opens against a blank "No web app yet" VM.
    if files.is_empty() {
        let key = workspace_key(&project);
        // #1312 — same LLM-first scaffold as project creation so an
        // automatically created project also starts from AI-generated code.
        // #1445 G3 — re-seed from the REAL stored intent (the creation
        // description rides in the project payload); `None` here used to
        // yield a generic brief that diverged from the user's prompt.
        let stored_intent = project
            .payload
            .get("description")
            .and_then(|d| d.as_str())
            .map(str::trim)
            .filter(|d| !d.is_empty());
        if let Err(e) = crate::scaffold::scaffold_project_workspace(
            &key,
            &project.name,
            &project.project_type,
            project.framework.as_deref(),
            stored_intent,
        )
        .await
        {
            log::warn!("Vibe run {}: seed empty workspace failed: {e}", project.name);
        } else {
            log::info!(
                "Vibe run {}: seeded empty workspace before run",
                project.name
            );
        }
    }
    let files = match crate::publish::collect_workspace_files(&project) {
        Ok(f) => f,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "success": false, "error": e }))),
    };
    let branch_id = resolve_org_branch(&registry, project.org_id).unwrap_or(project.branch_id);
    // #1371 — website projects NEVER provision a dev VM: they are static
    // HTMX/HTML pages served from the proxy container's shared websites dir
    // (same path as Deploy). Run stages the workspace into the project's two
    // proxy sites — the test twin `{slug}-test.{domain}` and, when the
    // request is approval-gated for production, the public `{slug}.{domain}`
    // — and returns the site URL directly. Only VM-backed kinds (`bot`,
    // `apps`) reach the `create_project_vm` path below.
    if project.project_type == "website" {
        return run_website_via_proxy(&registry, &project).await;
    }
    // #1504 — bot projects have their own two-env Run: recompile the
    // `{bot}-test` twin from the workspace (via the git-monitor hook) and
    // tell the UI to open the Chat window on the TEST tab. No dev VM.
    if project.project_type == "bot" {
        return match botcoresecrets::hooks::call_bot_project_ops("run-test", project_id) {
            Ok(()) => (
                StatusCode::OK,
                Json(serde_json::json!({
                    "success": true,
                    "bot_env": "test",
                    "bot": format!("{}-test", crate::bootstrap::bot_slug(&project.name)),
                    "project": project.name,
                })),
            ),
            Err(e) => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "success": false, "error": e })),
            ),
        };
    }
    let vm = match lifecycle.create_project_vm(
        project_id,
        branch_id,
        &project.name,
        &crate::vm_lifecycle::CreateVmRequest {
            env: "development".to_string(),
            tier: "small".to_string(),
            runner_enabled: false,
        },
    ) {
        Ok(vm) => vm,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "success": false, "error": e }))),
    };
    let port = query.port.unwrap_or_else(|| project_run_port(&project));
    // #1386 — the dev VM always gets the project's `_dev` database: dev Run
    // traffic can never read or write the production database.
    let database_url = match crate::project_db::ensure_project_database(
        registry.pool(),
        project.branch_id,
        &project.name,
        "test",
    ) {
        Ok(url) => Some(url),
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "success": false, "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botvibe::projects_api::workspace_2", None)})));
        }
    };
    match lifecycle.run_dev_app(&vm.container_name, &files, port, database_url.as_deref()) {
        Ok(url) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                // Same-origin route so the embedded Browser iframe can load
                // the dev VM through botserver (the generic /api/browser/proxy
                // rejects private hosts, and iframes cannot set headers).
                "url": format!("/api/vibe/projects/{project_id}/vm-preview?port={port}"),
                "host_url": url,
                "container": vm.container_name,
                "port": port,
                "project": project.name,
            })),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "success": false, "error": e })),
        ),
    }
}
