//! Host wiring for the AutoTask crate.
//!
//! The ops objects used to be declared *inside* the route builder, so nothing
//! outside `sub_router` could reach AutoTask: the chat command executor found
//! `tasks.autotask.create` in the catalog, called it, and got a deep link back
//! instead of an automation. They live here now so both the routes and the
//! command executor build the same `AutoTaskApi`.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use botautotask::types::{AutoTaskState, ConfigOps};
use botautotask::api::AutoTaskApi;
use botcore::shared::state::AppState;

/// Builds the AutoTask API for the running app state.
#[must_use]
pub fn build_api(app_state: &Arc<AppState>) -> Arc<AutoTaskApi> {
    struct AutoTaskStateImpl {
        pool: Arc<diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>>,
        bucket_name: String,
        manifests: Arc<RwLock<HashMap<String, botautotask::task_manifest::TaskManifest>>>,
        drive_ops: Option<Arc<dyn botautotask::types::DriveOps>>,
        /// Reform #1505 — writes generated sources into the bot's git
        /// repository (the canonical `.gbdialog`) instead of Drive.
        source_ops: Arc<dyn botautotask::types::BotSourceOps>,
        app_state: Arc<AppState>,
    }

    impl AutoTaskState for AutoTaskStateImpl {
        fn db_pool(&self) -> &botautotask::types::DbPool {
            &self.pool
        }
        fn bucket_name(&self) -> &str {
            &self.bucket_name
        }
        fn file_ops(&self) -> Option<&dyn botautotask::types::DriveOps> {
            self.drive_ops.as_deref()
        }
        fn source_ops(&self) -> Option<&dyn botautotask::types::BotSourceOps> {
            Some(self.source_ops.as_ref())
        }
        /// Reform #1505 — hand the compiler the closed keyword catalog, so
        /// the generated `basic_program` uses real BASIC instead of the
        /// `TALK` stub it fell back to when asked without any reference.
        fn basic_reference(&self) -> Option<String> {
            Some(crate::basic::keywords::keyword_reference::basic_keyword_reference())
        }
        fn broadcast_task_progress(&self, event: botautotask::types::TaskProgressEvent) {
            // #1266 — forward to the shared AppState channel so the
            // /ws/task-progress endpoint (and botui's proxy) receives
            // AutoTask progress events. Was previously a no-op.
            crate::main_module::routes::task_progress_ws::forward_autotask_event(
                &self.app_state,
                event,
            );
        }
        fn emit_activity(&self, _task_id: &str, _step: &str, _message: &str, _current: u8, _total: u8, _activity: botautotask::types::AgentActivity) {}
        fn emit_task_started(&self, _task_id: &str, _message: &str, _total_steps: u8) {}
        fn emit_task_error(&self, _task_id: &str, _step: &str, _error: &str) {}
        fn task_manifests(&self) -> &Arc<RwLock<HashMap<String, botautotask::task_manifest::TaskManifest>>> {
            &self.manifests
        }
        fn task_progress_broadcast(&self) -> Option<&tokio::sync::broadcast::Sender<botautotask::types::TaskProgressEvent>> {
            None
        }
    }

    struct ConfigOpsImpl {
        pool: Arc<diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>>,
    }

    impl ConfigOps for ConfigOpsImpl {
        // Real config resolution (per-bot Vault path → nil → global), so
        // AutoTask sees the same llm-model/llm-key as the chat pipeline. A
        // stub returning defaults made AutoTask fall back to gpt-4 and 404
        // against providers that do not host that model.
        fn get_config(&self, bot_id: &uuid::Uuid, key: &str, default: Option<&str>) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
            let manager = botcore::config::ConfigManager::new((*self.pool).clone());
            let value = manager
                .get_config(bot_id, key, default)
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { format!("config get {key}: {e}").into() })?;
            Ok(value)
        }
        fn set_config(&self, bot_id: &uuid::Uuid, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let manager = botcore::config::ConfigManager::new((*self.pool).clone());
            manager
                .set_config(bot_id, key, value)
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { format!("config set {key}: {e}").into() })?;
            Ok(())
        }
    }

let autotask_state = Arc::new(AutoTaskStateImpl {
        pool: Arc::new(app_state.conn.clone()),
        bucket_name: app_state.bucket_name.clone(),
        manifests: Arc::new(RwLock::new(HashMap::new())),
        drive_ops: app_state.drive.clone().map(|d| {
            Arc::new(botautotask::drive_ops::DriveRepositoryOps(d)) as Arc<dyn botautotask::types::DriveOps>
        }),
        source_ops: Arc::new(crate::main_module::git_bot_monitor::GitBotSourceOps::new(
            app_state.conn.clone(),
        )),
        app_state: app_state.clone(),
    });
    let config_ops = Arc::new(ConfigOpsImpl {
        pool: Arc::new(app_state.conn.clone()),
    });
    let llm_ops = Arc::new(botautotask::llm_adapter::BotlibLlmAdapter(app_state.llm_provider.clone()));
    Arc::new(AutoTaskApi::new(autotask_state, config_ops, llm_ops))
}
