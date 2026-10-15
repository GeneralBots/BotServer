//! `api::tools` — split per #1443 (AGENTS.md 450-line rule).

use super::*;

/// Upper bound on tool-call loops per run (#925).
pub(crate) const MAX_TOOL_CALLS: u32 = 500;

/// Heuristic for #1286: is this intent a MODifying turn (tools that write to
/// the project workspace) or a read-only query? Only modifying turns take
/// the project's exclusive edit lock; read-only queries stay parallel.
pub(crate) fn is_modifying_intent(intent: &str) -> bool {
    const MODIFY_HINTS: &[&str] = &[
        "create", "make", "build", "write", "generate", "develop", "add",
        "change", "update", "edit", "refactor", "fix", "remove", "delete",
        "rename", "deploy", "run", "modify", "implement", "replace", "move",
        "crie", "criar", "mude", "mudar", "atualize", "atualizar", "edite",
        "editar", "adicione", "adiccionar", "corrija", "corrigir", "remova",
        "remover", "implemente", "implementar",
    ];
    let lower = intent.to_lowercase();
    // Bounded scan: the intent is already capped at MAX_INTENT_CHARS.
    MODIFY_HINTS.iter().any(|h| {
        lower
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|w| w == *h)
    })
}

#[derive(Debug, Deserialize)]
pub struct CreateRunRequest {
    pub intent: String,
    /// Bot this run operates on. When absent, falls back to the default bot.
    /// The frontend passes the authenticated session's bot so runs are scoped
    /// to the user's tenant (not the global default).
    pub bot_id: Option<Uuid>,
    pub use_case: Option<String>,
    pub lang: Option<String>,
    pub auto_approve: Option<bool>,
    pub max_tool_calls: Option<u32>,
    pub timeout_seconds: Option<u64>,
    pub model: Option<String>,
    /// Vibe agent slot: `reasoning` | `agentic` | `fast`. Resolves the
    /// per-agent LLM provider configured in Settings → Vibe.
    pub agent: Option<String>,
    pub budget_cents: Option<u64>,
    /// vibe33 #811 — when "deploy", the run executes through the graph
    /// (PipelineEngine, approval-gated deploy pipeline) instead of the
    /// agent loop.
    pub pipeline_mode: Option<String>,
    /// Vibe project this run operates on (uuid string). When set, the
    /// deploy pipeline and the agent's harness tools resolve the project
    /// workspace instead of guessing from the intent text.
    pub project_id: Option<String>,
    /// Project name (workspace key) — the value the agent passes to
    /// file/run/git tools as `project`.
    pub project_name: Option<String>,
    /// Source-control mode for a project auto-created by this run:
    /// `native` (default), `git` (Forgejo-backed) or `github` (clone an
    /// external repository — see `clone_url`).
    pub source_control: Option<String>,
    /// External repository URL for `source_control = "github"` projects
    /// auto-created by this run.
    pub clone_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GetRunResponse {
    pub run_id: Uuid,
    pub bot_id: Uuid,
    pub session_id: Uuid,
    pub state: String,
    pub use_case: String,
    pub intent: String,
    pub tool_call_count: usize,
    /// #1394 — name of the most recent tool call, so chat/Runner Log can say
    /// WHAT the agent is doing instead of only counting calls.
    pub last_tool_name: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub error: Option<String>,
    /// Run budget in cents (from `VibeRunConfig`) so the Run Dock budget
    /// meter survives a page reload / persisted-run re-focus (#930).
    pub budget_cents: u64,
    pub lang: String,
    pub model: Option<String>,
    pub max_tool_calls: u32,
    pub auto_approve: bool,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    /// "deploy" when the run executed the production pipeline; None for
    /// development runs. Lets the UI skip dev-only actions (like opening
    /// the dev browser) after a deploy run (#1271).
    pub pipeline_mode: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListToolsResponse {
    pub tools: Vec<ToolDescriptor>,
}

pub(crate) struct VibeApiInner {
    pub(crate) state: Arc<dyn VibeState>,
    pub(crate) prompt_manager: Arc<VibePromptManager>,
    pub(crate) tool_executor: Arc<VibeToolExecutor>,
    pub(crate) telemetry: Arc<VibeTelemetry>,
    pub(crate) permissions: crate::permissions::PermissionEngineRef,
    pub(crate) skills: Arc<crate::skills::SkillStore>,
    pub(crate) runs: Arc<RwLock<HashMap<Uuid, VibeRun>>>,
    pub(crate) runs_store: crate::run_store::VibeRunStore,
    pub(crate) project_registry: ProjectRegistryRef,
    pub(crate) project_rbac: crate::rbac::ProjectRbac,
    /// #1286 — per-project edit locks: one exclusive write slot per project
    /// so parallel multi-chat sessions queue instead of interleaving writes.
    pub(crate) project_locks: Arc<crate::project_locks::ProjectLockRegistry>,
}

/// Bundled security dependencies (permissions + skills) wired into the Vibe
/// API, grouped so the router keeps a readable signature.
pub struct VibeSecurityDeps {
    pub permissions: crate::permissions::PermissionEngineRef,
    pub skills: Arc<crate::skills::SkillStore>,
}

/// Everything the Vibe API router is wired with, grouped so the router keeps a
/// short signature.
pub struct VibeApiDeps {
    pub state: Arc<dyn VibeState>,
    pub prompt_manager: Arc<VibePromptManager>,
    pub tool_executor: Arc<VibeToolExecutor>,
    pub telemetry: Arc<VibeTelemetry>,
    pub security: VibeSecurityDeps,
    pub pool: crate::types::DbPool,
    pub project_registry: ProjectRegistryRef,
    pub project_rbac: crate::rbac::ProjectRbac,
}

pub fn router(deps: VibeApiDeps) -> axum::Router {
    let VibeApiDeps {
        state,
        prompt_manager,
        tool_executor,
        telemetry,
        security,
        pool,
        project_registry,
        project_rbac,
    } = deps;
    let api = Arc::new(VibeApiInner {
        state,
        prompt_manager,
        tool_executor,
        telemetry,
        permissions: security.permissions,
        skills: security.skills,
        runs: Arc::new(RwLock::new(HashMap::new())),
        runs_store: crate::run_store::VibeRunStore::new(pool),
        project_registry,
        project_rbac,
        project_locks: Arc::new(crate::project_locks::ProjectLockRegistry::new()),
    });
    axum::Router::new()
        .route("/api/vibe/run", axum::routing::post(create_run))
        .route("/api/vibe/run/:run_id", axum::routing::get(get_run))
        .route("/api/vibe/run/:run_id/cancel", axum::routing::post(cancel_run))
        .route("/api/vibe/runs", axum::routing::get(list_runs))
        .route("/api/vibe/tools", axum::routing::get(list_tools))
        .route("/api/vibe/tools/:use_case", axum::routing::get(list_tools_for_use_case))
        .route("/api/vibe/metrics", axum::routing::get(get_global_metrics))
        .route("/api/vibe/metrics/:run_id", axum::routing::get(get_run_metrics))
        .route("/api/vibe/events/:run_id", axum::routing::get(get_run_events))
        .route("/api/vibe/run/:run_id/grounding", axum::routing::get(crate::grounding::get_run_grounding))
        .route("/api/vibe/run/:run_id/execute", axum::routing::post(execute_run))
        // #1290 — direct single-tool execution: the UI/REST callers invoke
        // `publish/project` etc. without spinning up a run. RBAC route rules
        // open POST /api/vibe/tools/** to authenticated users; deploy-grade
        // tools enforce roles + metering inside their handlers. The tool
        // name is a QUERY param (`?name=publish/project`) because axum's
        // matchit params never match `/`, and every vibe tool name contains
        // one (e.g. `publish/project`, `git/log`).
        .route("/api/vibe/tools/call", axum::routing::post(execute_tool_direct))
        .route("/api/vibe/graph/:use_case", axum::routing::get(crate::knowledge_graph::get_knowledge_graph))
        .route("/api/vibe/graph/run/:run_id", axum::routing::get(crate::knowledge_graph::get_run_graph))
        .route(
            "/api/vibe/projects/:project_id/conversation",
            axum::routing::get(export_project_conversation),
        )
        .route("/api/vibe/capabilities", axum::routing::get(list_capabilities))
        .route("/api/vibe/capabilities/:use_case", axum::routing::get(list_capabilities_for_use_case))
        .route("/api/vibe/pipeline/:use_case", axum::routing::get(get_pipeline))
        // #1288 — enterprise site lifecycle on the proxy container.
        .route("/api/vibe/projects/:project_id/site", axum::routing::delete(unpublish_project_site))
        .route("/api/vibe/projects/:project_id/site/rollback", axum::routing::post(rollback_project_site))
        // #1290 — promote the current DEV release to PROD; `?env=development`
        // rolls the DEV site back instead.
        .route("/api/vibe/projects/:project_id/site/promote", axum::routing::post(promote_project_site))
        .layer(axum::Extension(api))
}

#[derive(Debug, Deserialize)]
pub(crate) struct DirectToolRequest {
    #[serde(flatten)]
    pub(crate) arguments: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DirectToolParams {
    pub(crate) name: String,
}

/// #1290 — POST /api/vibe/tools/call?name=publish/project — execute a single
/// registered vibe tool directly (no run). The payload body is passed to the
/// tool as its arguments. Tools flagged `requires_approval` are refused here:
/// the direct path has no approval surface — callers go through the run flow
/// instead.
pub(crate) async fn execute_tool_direct(
    Extension(api): Extension<Arc<VibeApiInner>>,
    Extension(user): Extension<AuthenticatedUser>,
    Query(params): Query<DirectToolParams>,
    Json(body): Json<DirectToolRequest>,
) -> Response {
    let tool_name = params.name;
    // Audit line: only a short prefix of the user id goes to the log; the
    // full id remains on the call record (on_behalf_of_user) for tracing.
    info!("Vibe direct tool call '{tool_name}' by user {}", user.user_id.as_simple());
    // Honor the global permission mode exactly like the agent loop: in
    // Bypass mode destructive/deploy tools run without the approval gate;
    // otherwise they are refused here (no approval surface on this path).
    let mode = api.permissions.mode().await;
    let needs_approval = api.permissions.requires_approval(false, &tool_name, mode);
    let mut arguments = body.arguments;
    // #1280/#1291 — publish is deploy-role gated downstream; the tool runs
    // server-side without a session, so stamp the CALLING user's id into the
    // arguments exactly like the agent loop does (never silently privileged:
    // the deployment handler enforces the same RBAC for this id).
    if tool_name == "publish/project" {
        if let Some(args) = arguments.as_object_mut() {
            args.insert(
                "on_behalf_of_user".to_string(),
                serde_json::Value::String(user.user_id.to_string()),
            );
        }
    }
    let mut call = crate::types::VibeToolCall::new(
        Uuid::nil(),
        tool_name.clone(),
        arguments,
        needs_approval,
    );
    if matches!(mode, crate::permissions::PermissionMode::Bypass) {
        call.approved = true;
    }
    let result = api
        .tool_executor
        .execute(&mut call, crate::types::VibeUseCase::SoftwareDevelopment, api.state.as_ref())
        .await;
    match result {
        Ok(()) => {
            let payload = call
                .result
                .as_ref()
                .map(|r| r.data.clone())
                .unwrap_or_else(|| serde_json::json!({ "executed": true }));
            Json(serde_json::json!({ "success": true, "tool": tool_name, "result": payload }))
                .into_response()
        }
        Err(e) => {
            let needs_approval = call.requires_approval;
            warn!("Vibe direct tool '{tool_name}' failed: {e}");
            let status = if needs_approval {
                axum::http::StatusCode::ACCEPTED
            } else {
                axum::http::StatusCode::UNPROCESSABLE_ENTITY
            };
            (
                status,
                Json(serde_json::json!({
                    "success": false,
                    "tool": tool_name,
                    "error": e,
                    "requires_approval": needs_approval,
                })),
            )
                .into_response()
        }
    }
}

pub(crate) fn run_to_response(run: &VibeRun) -> GetRunResponse {
    GetRunResponse {
        run_id: run.run_id,
        bot_id: run.bot_id,
        session_id: run.session_id,
        state: run.state.to_string(),
        use_case: run.use_case.to_string(),
        intent: run.intent.clone(),
        tool_call_count: run.tool_calls.len(),
        last_tool_name: run.tool_calls.last().map(|c| c.tool_name.clone()),
        created_at: run.created_at.to_rfc3339(),
        completed_at: run.completed_at.map(|t| t.to_rfc3339()),
        error: run.error.clone(),
        budget_cents: run.config.budget_cents,
        lang: run.config.lang.clone(),
        model: run.config.model.clone(),
        max_tool_calls: run.config.max_tool_calls,
        auto_approve: run.config.auto_approve,
        project_id: run.config.project_id.clone(),
        project_name: run.config.project_name.clone(),
        pipeline_mode: run.config.pipeline_mode.clone(),
    }
}

/// #1190 — conversation export: every run for a project plus its tool-event
/// timeline, as one JSON document (chat/WhatsApp can attach it to a reply).
pub(crate) async fn export_project_conversation(
    Extension(api): Extension<Arc<VibeApiInner>>,
    axum::extract::Path(project_id): axum::extract::Path<Uuid>,
) -> impl IntoResponse {
    let want = project_id.to_string();
    let (live, persisted) = {
        let runs = api.runs.read().await;
        (runs.clone(), api.runs_store.list_runs(200))
    };
    let mut merged: Vec<VibeRun> = persisted;
    for run in live.into_values() {
        if let Some(existing) = merged.iter_mut().find(|r| r.run_id == run.run_id) {
            *existing = run;
        } else {
            merged.push(run);
        }
    }
    merged.sort_by_key(|r| r.created_at);

    let mut conversation: Vec<serde_json::Value> = Vec::new();
    for run in merged {
        if run.config.project_id.as_deref() != Some(&want) {
            continue;
        }
        let events = api.telemetry.get_events_for_run(run.run_id, 200).await;
        let run_json = match serde_json::to_value(&run) {
            Ok(v) => v,
            Err(e) => {
                log::error!("Vibe conversation export: serialize run: {e}");
                continue;
            }
        };
        let events_json = match serde_json::to_value(&events) {
            Ok(v) => v,
            Err(e) => {
                log::error!("Vibe conversation export: serialize events: {e}");
                serde_json::Value::Array(Vec::new())
            }
        };
        conversation.push(serde_json::json!({
            "run": run_json,
            "events": events_json,
        }));
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "project_id": project_id,
            "conversation": conversation,
        })),
    )
}

pub(crate) async fn list_tools(Extension(api): Extension<Arc<VibeApiInner>>) -> impl IntoResponse {
    let tools = api.tool_executor.registry().list_tools().await;
    Json(ListToolsResponse { tools })
}

pub(crate) async fn list_capabilities(Extension(api): Extension<Arc<VibeApiInner>>) -> impl IntoResponse {
    let tools = api.tool_executor.registry().list_tools().await;
    let capabilities = crate::capability_registry::build_capabilities(&tools);
    Json(CapabilitiesResponse {
        success: true,
        capabilities,
        error: None,
    })
}
