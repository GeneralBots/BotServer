//! background - extracted from bootstrap.rs

use botcore::shared::state::AppState;
use crate::core::bot::BotOrchestrator;
use crate::llm::local::ensure_llama_servers_running;
use log::{error, info, trace};
use std::sync::Arc;

use super::drive_monitors::{start_drive_monitors, start_drive_compiler};

#[cfg(feature = "drive")]
use super::git_bot_monitor::{
    run_archive_pass, run_import_pass, start as start_git_bot_monitor,
};

/// Runs every 5 minutes and promotes trialing `billing_recurring` rows whose
/// trial period has ended to `active` at the plan price, generating the first
/// invoice (issue #778). The signup flow writes trials straight to the DB, so
/// this DB-driven job is the only path that converts them to paid.
fn start_trial_promotion_guard(app_state: Arc<AppState>) {
    #[cfg(feature = "billing")]
    {
        let pool = app_state.conn.clone();
        tokio::spawn(async move {
            info!("Billing trial promotion guard started (every 5 minutes)");
            loop {
                match pool.get() {
                    Ok(mut conn) => {
                        if let Err(e) = botbilling::lifecycle::promote_expired_trials_in_db(&mut conn) {
                            error!("Billing trial promotion failed: {}", e);
                        }
                    }
                    Err(e) => error!("Billing trial promotion pool error: {}", e),
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
            }
        });
    }
    #[cfg(not(feature = "billing"))]
    let _ = app_state;
}


pub async fn start_background_services(
  app_state: Arc<AppState>,
  _pool: &botcore::shared::utils::DbPool,
) {
    use botcore::shared::memory_monitor::{log_process_memory, start_memory_monitor};

    // Resume workflows after server restart
    if let Err(e) =
        crate::basic::keywords::orchestration::resume_workflows_on_startup(Arc::new(crate::basic::AppStateBasicRuntime(app_state.clone()))).await
    {
        log::warn!("Failed to resume workflows on startup: {}", e);
    }

    #[cfg(feature = "tasks")]
    {
        let tasks_state = Arc::new(crate::tasks::TasksState {
            pool: app_state.conn.clone(),
            run_command: Arc::new(|_cmd: &str, _args: &[&str]| -> Result<String, String> {
                Ok("stub".to_string())
            }),
            call_llm: Arc::new(|_sys: &str, _prompt: &str| -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send>> {
                Box::pin(async { Ok("stub".to_string()) })
            }),
            get_config: Arc::new(|_key: &str| -> Result<String, String> {
                Ok("stub".to_string())
            }),
            cache_get: Arc::new(|_key: String| -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<String>, String>> + Send>> {
                Box::pin(async { Ok(None) })
            }),
            cache_set: Arc::new(|_key: String, _value: String, _ttl: Option<u64>| -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>> {
                Box::pin(async { Ok(()) })
            }),
        });
        let task_scheduler = Arc::new(crate::tasks::scheduler::TaskScheduler::new(tasks_state));
        task_scheduler.start();
    }

    #[cfg(any(feature = "research", feature = "llm"))]
    if let Err(e) = crate::core::kb::ensure_crawler_service_running(app_state.clone()).await {
        log::warn!("Failed to start website crawler service: {}", e);
    }

    // Start memory monitoring - check every 30 seconds, warn if growth > 50MB
    start_memory_monitor(30, 50);
    info!("Memory monitor started");
    log_process_memory();

    let bot_orchestrator = BotOrchestrator::new(app_state.clone());
    if let Err(e) = bot_orchestrator.mount_all_bots() {
        error!("Failed to mount bots: {}", e);
    }

    #[cfg(feature = "llm")]
    {
        let app_state_for_llm = app_state.clone();
        trace!("ensure_llama_servers_running starting...");
        if let Err(e) = ensure_llama_servers_running(app_state_for_llm).await {
            error!("Failed to start LLM servers: {}", e);
        }
        trace!("ensure_llama_servers_running completed");
    }

  // Start DriveMonitor for S3/MinIO file watching and syncing
  #[cfg(feature = "drive")]
  start_drive_monitors(app_state.clone(), _pool).await;

  // Start DriveCompiler to compile .bas files from drive_files table
  #[cfg(feature = "drive")]
  start_drive_compiler(app_state.clone()).await;

  // Reform #1500/#1501/#1502 — Vibe git bot monitor: bot sources come from
  // Forgejo (one repo per project in the branch org) instead of Drive. The
  // import pass moves legacy Drive sources into git once (idempotent), then
  // the monitor git-pulls into work/ and feeds the existing compile pipeline.
  #[cfg(all(feature = "drive", feature = "vibe"))]
  {
    let pool_for_vibe = _pool.clone();
    let state_for_import = app_state.clone();
    tokio::spawn(async move {
      botvibe::bootstrap_backfill::backfill_default_projects(pool_for_vibe.clone()).await;
      run_import_pass(state_for_import.clone(), pool_for_vibe.clone()).await;
      // #1501 step 3 — once imports are verified, move the legacy Drive
      // source prefixes to the archive prefix (reversible, never deleted).
      run_archive_pass(state_for_import, pool_for_vibe).await;
    });
    start_git_bot_monitor(app_state.clone(), _pool.clone()).await;
    register_vibe_bootstrap_hook(_pool.clone());
  }

  // Start billing trial promotion (trialing -> active + first invoice)
  // Channel event subscriptions (#1507): tools declared with
  // `ON EVENT "…" CALL "…"` run when a channel publishes the event.
  crate::main_module::event_dispatcher::start_event_dispatcher(app_state.clone());

  start_trial_promotion_guard(app_state.clone());
    // start_config_watcher(app_state.clone()).await;
}

/// Reform #1500 — signup hook: creates the branch's default-bot Vibe project
/// (and its PROD/TEST bot rows) when `create_cloud_workspace_inner` fires in
/// botcloud.botcloud cannot depend on botvibe, so the main binary registers
/// this closure and the signup flow invokes it through the hook registry.
#[cfg(all(feature = "drive", feature = "vibe"))]
fn register_vibe_bootstrap_hook(pool: botcore::shared::utils::DbPool) {
  let hook: botcoresecrets::hooks::WorkspaceBootstrapHook =
    std::sync::Arc::new(move |branch_id, name| {
      botvibe::bootstrap::ensure_branch_default_project(&pool, branch_id, branch_id, &name)
        .map(|_| ())
    });
  botcoresecrets::hooks::register_workspace_bootstrap_hook(hook);
}
