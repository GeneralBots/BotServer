use super::*;

pub async fn configure_vibe_routes(app_state: &Arc<AppState>) -> axum::Router {
    let pool = app_state.conn.clone();
    // #1444 M2 — the skills store shares the pool (hydrate + write-through);
    // cloned BEFORE the pool moves into the VibeState below.
    let skills_pool = pool.clone();

    // Reform #1500 — share the pool with the bootstrap helpers so org/naming
    // resolution works without threading pools through every API surface.
    botvibe::bootstrap_backfill::init_shared_pool(pool.clone());
    // Reform #1500 — boot backfill: every branch gets its default-bot Vibe
    // project + PROD/TEST bot pair (idempotent, runs on every boot).
    {
        let pool_for_backfill = pool.clone();
        tokio::spawn(async move {
            botvibe::bootstrap_backfill::backfill_default_projects(pool_for_backfill).await;
        });
    }

    // Reform #1500 — share the pool with the bootstrap helpers so org/naming
    // resolution works without threading pools through every API surface.
    botvibe::bootstrap_backfill::init_shared_pool(pool.clone());
    // Reform #1500 — boot backfill: every branch gets its default-bot Vibe
    // project + PROD/TEST bot pair (idempotent, runs on every boot).
    {
        let pool_for_backfill = pool.clone();
        tokio::spawn(async move {
            botvibe::bootstrap_backfill::backfill_default_projects(pool_for_backfill).await;
        });
    }

    let state: Arc<dyn VibeState> = Arc::new(VibeStateImpl {
        pool,
        progress: Some(broadcast::channel(128).0),
        runs: Arc::new(RwLock::new(HashMap::new())),
        signals: Some(broadcast::channel(128).0),
        config: ConfigManager::new(app_state.conn.clone()),
    });

    let registry = ProjectRegistry::new(app_state.conn.clone());
    match registry.ensure_schema() {
        Ok(()) => info!("Vibe: vibe_projects schema ensured"),
        Err(e) => log::error!("Vibe: ensure vibe_projects schema failed: {e}"),
    }
    let project_registry = Arc::new(registry);

    let vm_lifecycle = Arc::new(VmLifecycle::new(app_state.conn.clone()));
    match vm_lifecycle.ensure_schema() {
        Ok(()) => info!("Vibe: vm_instances schema ensured"),
        Err(e) => log::error!("Vibe: ensure vm_instances schema failed: {e}"),
    }

    // #1181/#1167 — Agent VM idle reaper + expiry sweep. Runs once at boot
    // (expiry sweep for orphaned VMs from crashed runs) and then every 10
    // minutes: stops VMs idle > 30 min, deletes VMs older than 7 days.
    {
        let lifecycle = vm_lifecycle.clone();
        tokio::spawn(async move {
            let idle_secs: i64 = std::env::var("GB_VIBE_VM_IDLE_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30 * 60);
            let max_age_secs: i64 = std::env::var("GB_VIBE_VM_MAX_AGE_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(7 * 24 * 3600);
            let run = |label: &str| match lifecycle.reap(idle_secs, max_age_secs) {
                Ok(reaped) if !reaped.is_empty() => {
                    info!("Vibe VM reaper ({label}): {}", reaped.join(", "));
                }
                Ok(_) => {}
                Err(e) => log::error!("Vibe VM reaper ({label}) failed: {e}"),
            };
            run("boot sweep");
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(600));
            loop {
                tick.tick().await;
                run("periodic");
            }
        });
    }

    // #1271 — Prod-VM guard: production VMs must stay running forever once
    // deployed. A server (or host) restart can leave deployed containers
    // stopped; this job reconciles them back to `running` at boot and every
    // 60 seconds. Dev/staging VMs are lifecycle-managed by the frontend
    // (start on Run, stop on window close / project switch) and are NOT
    // touched here.
    {
        let lifecycle = vm_lifecycle.clone();
        tokio::spawn(async move {
            let run = |label: &str| match lifecycle.ensure_prod_running() {
                Ok(started) if !started.is_empty() => {
                    info!(
                        "Vibe prod-VM guard ({label}): (re)started {}",
                        started.join(", ")
                    );
                }
                Ok(_) => {}
                Err(e) => log::error!("Vibe prod-VM guard ({label}) failed: {e}"),
            };
            // Give the DB pool and Incus a moment at boot before probing.
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            run("boot");
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                tick.tick().await;
                run("periodic");
            }
        });
    }

    let domain_binds = Arc::new(ProjectDomains::new(app_state.conn.clone()));
    match domain_binds.ensure_schema() {
        Ok(()) => info!("Vibe: project_domains schema ensured"),
        Err(e) => log::error!("Vibe: ensure project_domains schema failed: {e}"),
    }

    let prompt_manager = Arc::new(VibePromptManager::new());
    let permissions = Arc::new(botvibe::PermissionEngine::new());
    // #1444 M2 — persistent skills: the store hydrates the `vibe_skills`
    // table (custom skills + marketplace installs survive restarts) and
    // writes through on register/delete; bootstrap seeding stays idempotent.
    let skills = Arc::new(botvibe::SkillStore::with_persistence(skills_pool));
    if let Err(e) = skills.seed_bootstrap().await {
        log::error!("Vibe: bootstrap skills seeding failed: {e}");
    }
    // #816 — write-through persistence so canvases/issues/sessions/teams
    // survive server restarts (previously in-memory RwLock<Vec> only).
    let canvases = Arc::new(botvibe::CanvasStore::with_persistence(app_state.conn.clone()));
    let issues = Arc::new(botvibe::IssueStore::with_persistence(app_state.conn.clone()));
    let sessions = Arc::new(botvibe::SessionStore::with_persistence(app_state.conn.clone()));
    let teams = Arc::new(botvibe::TeamStore::with_persistence(app_state.conn.clone()));

    let tool_registry = Arc::new(ToolRegistry::new());
    if let Err(e) = tool_registry
        .register_m5_tools(skills.clone(), canvases.clone(), issues.clone())
        .await
    {
        log::error!("Vibe: M5 tool registration failed: {e}");
    }
    // VIBE_SCHEMA persistence (Issue #793): ensure tables at boot.
    match botvibe::run_store::ensure_vibe_schema(&app_state.conn) {
        Ok(()) => info!("Vibe: vibe_runs schema ensured"),
        Err(e) => log::error!("Vibe: ensure vibe_runs schema failed: {e}"),
    }
    let run_store = botvibe::run_store::VibeRunStore::new(app_state.conn.clone());
    let recovered = run_store.recover_orphaned_runs();
    if recovered > 0 {
        info!("Vibe: marked {recovered} orphaned run(s) as failed after restart");
    }
    let tool_executor = Arc::new(VibeToolExecutor::new(tool_registry));
    let telemetry = Arc::new(VibeTelemetry::with_persistence(app_state.conn.clone()));

    let vm_ops = Arc::new(VmOps::new(app_state.conn.clone()));
    let backups = Arc::new(Backups::new(app_state.conn.clone()));
    match backups.ensure_schema() {
        Ok(()) => info!("Vibe: vm_backups schema ensured"),
        Err(e) => log::error!("Vibe: ensure vm_backups schema failed: {e}"),
    }

    let project_rbac = ProjectRbac::new(app_state.conn.clone());
    match project_rbac.ensure_schema() {
        Ok(()) => info!("Vibe: project_members schema ensured"),
        Err(e) => log::error!("Vibe: ensure project_members schema failed: {e}"),
    }

    // AI-OS wave (#1171/#1172/#1173/#1175/#1182/#1185): in-memory run
    // stores (same model as the pre-#816 canvases/issues) exposed through
    // the routers merged above.
    let planner = Arc::new(botvibe::PlannerExecutor::new());
    let agents = Arc::new(botvibe::AgentRegistry::new());
    let moa = Arc::new(botvibe::MoaEngine::new());
    let browser_memory = Arc::new(botvibe::BrowserMemory::new());
    let browser_driver = Arc::new(botvibe::BrowserDriver::new());

    // #1185 — Proactivity scheduler. Seeds a couple of consented demo
    // triggers, then ticks every 60s emitting suggestion cards that the
    // desktop Notification Center polls via /api/vibe/proactivity/cards.
    let proactivity = Arc::new(botvibe::ProactivityEngine::new());
    {
        use botvibe::proactivity::RegisterTriggerRequest;
        let seeds = [
            RegisterTriggerRequest {
                category: "daily-briefing".to_string(),
                description: "Morning summary of open projects and runs".to_string(),
                interval_secs: 3600,
                consent_required: true,
                consented: true,
            },
            RegisterTriggerRequest {
                category: "vm-health".to_string(),
                description: "Idle or over-budget VMs reminder".to_string(),
                interval_secs: 1800,
                consent_required: true,
                consented: false,
            },
        ];
        for seed in seeds {
            let trigger = proactivity.register(&seed).await;
            info!("Vibe proactivity: seeded trigger '{category}'", category = trigger.category);
        }
    }
    {
        let engine = proactivity.clone();
        let briefing_pool = app_state.conn.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                ticker.tick().await;
                let emit = |t: &botvibe::proactivity::TriggerDef| {
                    // #1191 — the daily briefing card carries real per-app
                    // history + LLM relevance instead of a static stub.
                    if t.category == "daily-briefing" {
                        return Some(botvibe::daily_briefing::build_daily_briefing_blocking(
                            &briefing_pool,
                        ));
                    }
                    Some(format!(
                        "{} — {}",
                        t.category, t.description
                    ))
                };
                match engine.tick(&emit).await {
                    0 => {}
                    n => info!("Vibe proactivity: emitted {n} suggestion card(s)"),
                }
            }
        });
    }

    let metering = Arc::new(VMetering::new(app_state.conn.clone()));
    match metering.ensure_schema() {
        Ok(()) => info!("Vibe: vm_metering schema ensured"),
        Err(e) => log::error!("Vibe: ensure vm_metering schema failed: {e}"),
    }
    VMetering::spawn_vm_hours_sampler(metering.clone(), 3600);
    info!("Vibe: vm-hours metering sampler started (hourly)");

    botvibe::api::router(
        state.clone(),
        prompt_manager.clone(),
        tool_executor.clone(),
        telemetry.clone(),
        botvibe::api::VibeSecurityDeps {
            permissions: permissions.clone(),
            skills: skills.clone(),
        },
        app_state.conn.clone(),
        project_registry.clone(),
        project_rbac.clone(),
    )
        .merge(projects_router(
            project_registry.clone(),
            project_rbac.clone(),
            metering.clone(),
            vm_lifecycle.clone(),
        ))
        .merge(vms_router(
            vm_lifecycle,
            project_registry.clone(),
            project_rbac.clone(),
            metering.clone(),
        ))
        .merge(domains_router(domain_binds, project_rbac.clone(), metering.clone()))
        .merge(botvibe::publish::publish_router(app_state.conn.clone()))
        .merge(botvibe::domain_auth::domain_auth_router(app_state.conn.clone()))
        .merge(members_router(project_rbac.clone()))
        .merge(metering_router(metering.clone(), project_rbac.clone()))
        .merge(ops_router(OpsRoutes {
            vm_ops,
            backups,
            registry: project_registry,
            pool: app_state.conn.clone(),
            rbac: project_rbac,
            metering: metering.clone(),
        }))
        .merge(botvibe::permissions_router(permissions.clone()))
        .merge(botvibe::skills_router(skills.clone()))
        .merge(botvibe::doctor_router(skills.clone(), sessions.clone()))
        .merge(botvibe::canvases_router(canvases.clone()))
        .merge(botvibe::issues_router(issues.clone()))
        .merge(botvibe::sessions_router(botvibe::SessionRoutes {
            sessions: sessions.clone(),
            state: state.clone(),
            prompt_manager: prompt_manager.clone(),
            tool_executor: tool_executor.clone(),
            telemetry: telemetry.clone(),
            permissions: permissions.clone(),
            skills: skills.clone(),
        }))
        .merge(botvibe::teams_router(botvibe::TeamRoutes {
            teams: teams.clone(),
            state: state.clone(),
            prompt_manager: prompt_manager.clone(),
            tool_executor: tool_executor.clone(),
            telemetry: telemetry.clone(),
            permissions: permissions.clone(),
            skills: skills.clone(),
        }))
        // AI-OS wave (#1171/#1172/#1173/#1175/#1182/#1185): planner,
        // agent API, mixture-of-agents, browsing memory, browser driver
        // and proactivity scheduler — all mounted under /api/vibe/**.
        .merge(botvibe::planner_router(planner))
        .merge(botvibe::agents_router(agents))
        .merge(botvibe::moa_router(moa))
        .merge(botvibe::browser_memory_router(browser_memory))
        .merge(botvibe::browser_driver_router(browser_driver))
        .merge(botvibe::proactivity_router(proactivity))
}
