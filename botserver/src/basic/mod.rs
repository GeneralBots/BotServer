use botcore::shared::state::AppState;
use botbasic_comms::keywords::universal_messaging::register_universal_messaging;
use botbasic_comms::keywords::send_mail::send_mail_keyword;
use log::{info, warn};
use rhai::{Dynamic, Engine, EvalAltResult, Scope};
pub use botcore::shared::UserSession;

pub use botbasic_compiler as compiler;
pub mod keywords;
mod runtime_llm;


#[derive(Debug)]
pub struct ScriptService {
    pub engine: Engine,
    pub scope: Scope<'static>,
    pub role: String,
}

impl ScriptService {
    #[must_use]
    pub fn new(state: Arc<AppState>, user: LocalUserSession) -> Self {
        let mut engine = Engine::new();
        let _ = engine.set_max_operations(50_000);
        let _ = engine.set_max_call_levels(10);
        let scope = Scope::new();
        engine.set_allow_anonymous_fn(true);
        engine.set_allow_looping(true);

        let role = crate::security::user_role::resolve_user_role(&state.conn, user.user_id);

        let state_for_local = state.clone();
        let runtime: Arc<dyn BasicRuntime> = Arc::new(AppStateBasicRuntime(state));
        let bt_user = botbasic_types::UserSession::from(user.clone());

        botbasic_core::register_core_keywords(runtime.clone(), bt_user.clone(), &mut engine);
        botbasic_data::register_data_keywords(runtime.clone(), bt_user.clone(), &mut engine);
        botbasic_comms::register_comms_keywords(&runtime, bt_user.clone(), &mut engine);
        botbasic_ai::register_ai_keywords(runtime.clone(), bt_user.clone(), &mut engine);
        botbasic_system::register_system_keywords(runtime, bt_user, &mut engine);

        // Register local (botserver-only) keywords that need Arc<AppState>
        keywords::set_answer_mode::register_set_answer_mode_keyword(state_for_local.clone(), user.clone(), &mut engine);
        register_universal_messaging(state_for_local.clone(), user.clone(), &mut engine);
        send_mail_keyword(state_for_local.clone(), user.clone(), &mut engine);
        keywords::vibe_agent::register_vibe_run_keyword(&mut engine);
        keywords::vibe_agent::register_vibe_status_command(&mut engine);
        keywords::vibe_agent::register_vibe_approve_command(&mut engine);
        keywords::vibe_agent::register_vibe_cancel_command(&mut engine);
        keywords::vibe_agent::register_vibe_tools_command(&mut engine);
        keywords::vibe_agent::register_vibe_events_command(&mut engine);
        keywords::invoke_action::register_invoke_keywords(&mut engine);

        Self { engine, scope, role }
    }

    pub fn inject_config_variables(&mut self, config_vars: HashMap<String, String>) {
        for (key, value) in config_vars {
            let var_name = if key.starts_with("param-") {
                key.strip_prefix("param-").unwrap_or(&key).to_lowercase()
            } else {
                key.to_lowercase()
            };

            if let Ok(int_val) = value.parse::<i64>() {
                self.scope.push(&var_name, int_val);
            } else if let Ok(float_val) = value.parse::<f64>() {
                self.scope.push(&var_name, float_val);
            } else if value.eq_ignore_ascii_case("true") {
                self.scope.push(&var_name, true);
            } else if value.eq_ignore_ascii_case("false") {
                self.scope.push(&var_name, false);
            } else {
                self.scope.push(&var_name, value);
            }
        }
    }

    pub fn load_bot_config_params(&mut self, state: &AppState, bot_id: uuid::Uuid) {
        use botcore::config::ConfigManager;
        let cfg = ConfigManager::new(state.conn.clone());
        let all = cfg.get_all_config(&bot_id);
        let params: HashMap<String, String> = all
            .into_iter()
            .filter(|(k, _)| k.starts_with("param-"))
            .collect();
        if !params.is_empty() {
            self.inject_config_variables(params);
        }
    }

    pub fn run(&mut self, ast_content: &str) -> Result<Dynamic, Box<EvalAltResult>> {
        self.scope.set_or_push("ROLE", Dynamic::from(self.role.clone()));
        self.scope.set_or_push("role", Dynamic::from(self.role.clone()));
        if let Err(e) = std::fs::write("/tmp/run_ast_input.txt", ast_content) {
            log::warn!("Failed to write /tmp/run_ast_input.txt: {}", e);
        }
        let preprocessed = compiler::syntax_transforms::convert_multiword_keywords(ast_content);
        if let Err(e) = std::fs::write("/tmp/run_preprocessed.txt", &preprocessed) {
            log::warn!("Failed to write /tmp/run_preprocessed.txt: {}", e);
        }

        #[cfg(feature = "security")]
        {
            use botsecurity::protection::script_guard::{
                AuditLogger, InspectionContext, ScriptGuard,
            };
            let ctx = InspectionContext::default();
            let result = ScriptGuard::inspect(&preprocessed, &ctx);
            if !result.allowed {
                AuditLogger::log_blocked(&result);
                log::error!(
                    "[SECURITY] Script blocked — {} dangerous patterns detected",
                    result.blocked_patterns.len()
                );
                return Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!(
                        "Script blocked by security guard: {} dangerous patterns detected",
                        result.blocked_patterns.len()
                    )
                    .into(),
                    rhai::Position::NONE,
                )));
            }
        }

        let ast = match self.engine.compile(&preprocessed) {
            Ok(ast) => ast,
            Err(e) => {
                if let Err(we) = std::fs::write("/tmp/compile_error.txt", format!("compile error: {}\npreprocessed lines: {}\n", e, preprocessed.lines().count())) {
                    log::warn!("Failed to write /tmp/compile_error.txt: {}", we);
                }
                log::error!("[BASIC_EXEC] Failed to compile AST: {}", e);
                return Err(Box::new(e.into()));
            }
        };
        self.engine.eval_ast_with_scope(&mut self.scope, &ast)
    }

    pub async fn execute_script(
        state: Arc<AppState>,
        user: LocalUserSession,
        ast_content: &str,
    ) -> Result<String, String> {
        std::fs::write("/tmp/execute_script_debug.txt", format!(
            "ast_content len={}\nfirst 200: {}\n", ast_content.len(), ast_content.chars().take(200).collect::<String>()
        )).ok();
        let ast_owned = ast_content.to_string();
        let bot_id = user.bot_id;
        let state_outer = state.clone();
        let user_outer = user.clone();
        let result = tokio::task::spawn_blocking(move || {
            let state_for_svc = state_outer.clone();
            let mut script_service = ScriptService::new(state_outer, user_outer.clone());
            script_service.load_bot_config_params(&state_for_svc, bot_id);
            if let Some(obj) = user_outer.context_data.as_object() {
                for key in &["trigger_record_id", "trigger_old_status", "trigger_new_status", "channel"] {
                    if let Some(val) = obj.get(*key).and_then(|v| v.as_str()) {
                        let rhai_key = key.to_uppercase();
                        let _ = script_service.set_variable(&rhai_key, val);
                    }
                }
            }
            script_service.run(&ast_owned)
        }).await.map_err(|e| format!("spawn_blocking join error: {}", e))?;
        std::fs::write("/tmp/run_result.txt", format!("run result: {:?}\n", result)).ok();
        match result {
            Ok(result) => {
                let s = result.to_string();
                std::fs::write("/tmp/talk_debug.txt", format!("TALK debug: result='{}' len={}\n", s, s.len())).ok();
                Ok(s)
            }
            Err(e) => {
                let err_str = format!("Script error: {}", e);
                std::fs::write("/tmp/talk_debug.txt", format!("TALK ERROR: {}\n", err_str)).ok();
                Err(err_str)
            }
        }
    }

    pub fn set_variable(&mut self, name: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.scope.set_or_push(name, Dynamic::from(value.to_string()));
        Ok(())
    }
}

#[cfg(test)]
pub mod tests;

use std::collections::HashMap;
use std::sync::Arc;
use botbasic_types::BasicRuntime;
use botcore::shared::UserSession as LocalUserSession;
use uuid::Uuid;

#[derive(Debug)]
pub struct AppStateBasicRuntime(pub Arc<botcore::shared::state::AppState>);

impl BasicRuntime for AppStateBasicRuntime {
    fn db_pool(&self) -> &botlib::db_pool::DbPool {
        &self.0.conn
    }

    fn cache_client(&self) -> Option<Arc<redis::Client>> {
        #[cfg(feature = "cache")]
        {
            self.0.cache.clone()
        }
        #[cfg(not(feature = "cache"))]
        {
            None
        }
    }

    fn bucket_name(&self) -> &str {
        &self.0.bucket_name
    }

    fn hear_channels(&self) -> &std::sync::Mutex<HashMap<Uuid, std::sync::mpsc::SyncSender<String>>> {
        &self.0.hear_channels
    }

    fn bot_database_manager(&self) -> Arc<dyn botlib::traits::BotDatabaseService> {
        Arc::clone(&self.0.bot_database_manager)
    }

    fn web_adapter(&self) -> Arc<dyn botlib::traits::ChannelAdapter> {
        Arc::clone(&self.0.web_adapter)
    }

    fn drive_repository(&self) -> Option<Arc<dyn botlib::traits::DriveRepository>> {
        self.0.drive.clone()
    }

    fn config_value(&self, key: &str) -> Option<String> {
        if let Some(v) = self.0.config.as_ref()?.get(key) {
            return Some(v.clone());
        }
        // Environment fallback, mirroring ConfigManager::get_config: keys
        // like `botmodels-host` resolve from `BOTMODELS_HOST`. Without this
        // the tool runtime could never see process-level configuration
        // (multimodal clients were permanently disabled in deployments that
        // provision BotModels through systemd environment).
        let env_key = key.to_uppercase().replace('-', "_");
        if let Ok(val) = std::env::var(&env_key) {
            if !val.is_empty() {
                return Some(val);
            }
        }
        // #1425 — external connection credentials (`conn-{bot}-{name}-{Field}`
        // keys, e.g. `conn-*-Password`) resolve from the per-bot Vault path
        // through the ConfigManager when the session config map has no value.
        // The drive monitor drops sensitive keys from config.csv, so Vault is
        // the only place a connection password can live; without this
        // fallback the TABLE keyword could never authenticate a
        // Vault-provisioned connection. The bot id is parsed from the key
        // prefix itself (the runtime carries no bot_id field).
        let rest = key.strip_prefix("conn-")?;
        let bot_str = rest.split('-').next()?;
        let bot_id = uuid::Uuid::parse_str(bot_str).ok()?;
        let manager = botcore::config::ConfigManager::new(self.0.conn.clone());
        manager.get_config(&bot_id, key, None).ok().filter(|v| !v.is_empty())
    }

    fn session_manager(&self) -> Arc<tokio::sync::Mutex<dyn botlib::traits::SessionManagerService>> {
        Arc::clone(&self.0.session_manager)
    }

    fn llm_generate(&self, prompt: &str, model: &str, api_key: &str) -> Result<String, String> {
        // Tool scripts (classify_media, etc.) reach the model through this
        // trait. `runtime_llm` owns the per-bot Vault resolution, the
        // sync/async bridge and the model-handler post-processing (#1465).
        runtime_llm::generate(&self.0, prompt, model, api_key)
    }

    fn default_bot_id(&self) -> String {
        runtime_llm::default_bot_uuid(&self.0).to_string()
    }

    fn update_session_user(&self, session_id: Uuid, user_id: Uuid) -> Result<(), String> {
        let sm = Arc::clone(&self.0.session_manager);
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            if let Ok(rt) = rt {
                let result = rt.block_on(async {
                    let mut guard = sm.lock().await;
                    guard.update_user_id(session_id, user_id)
                });
                let _ = tx.send(result);
            }
        });
        rx.recv().unwrap_or(Err("Channel error".to_string()))
    }

    fn send_message(&self, response: &botlib::models::BotResponse) -> Result<(), String> {
        let sid = response.session_id.clone();
        let resp = response.clone();
        let channels = self.0.response_channels.clone();
        let bot_id = response.bot_id.clone();
        let content_preview = response.content.chars().take(60).collect::<String>();
        info!("send_message: attempting to send to session={}, bot={}, content='{}'",
            sid, bot_id, content_preview);

        let mut channel_found = false;
        {
            let guard = channels.blocking_lock();
            let count = guard.len();
            if let Some(tx) = guard.get(&sid) {
                info!("send_message: FOUND channel for session {} ({} channels total)", sid, count);
                if let Err(e) = tx.try_send(resp) {
                    warn!("send_message: try_send failed for session {}: {}", sid, e);
                }
                channel_found = true;
            } else {
                let keys: Vec<&String> = guard.keys().collect();
                let matched_key = keys.iter().find(|k| k.starts_with(&format!("{}_", sid)));
                if let Some(key) = matched_key {
                    if let Some(tx) = guard.get(*key) {
                        info!("send_message: FOUND channel via prefix match for session {} (key={})", sid, key);
                        if let Err(e) = tx.try_send(resp) {
                            warn!("send_message: try_send failed for session {}: {}", sid, e);
                        }
                        channel_found = true;
                    }
                }
                if !channel_found {
                    warn!("send_message: NO channel for session {} ({} channels total, keys: {:?})",
                        sid, count, keys);
                }
            }
        }

        // The reply outlived its channel — a tool triggered by `ON EVENT`
        // runs after the inbound request ended. Hand it to the channel
        // adapter so the conversation still receives it.
        if !channel_found
            && crate::main_module::channel_delivery::deliver_via_channel(&self.0, response)
        {
            info!("send_message: delivered session {} through the {} adapter", sid, response.channel);
        }
        Ok(())
    }

    fn execute_script(&self, user: botbasic_types::UserSession, script: &str) -> Result<String, String> {
        let lib_user = botcore::shared::UserSession {
            id: user.id,
            user_id: user.user_id,
            branch_id: user.branch_id,
            bot_id: user.bot_id,
            title: user.title.clone(),
            context_data: user.context_data.clone(),
            current_tool: user.current_tool.clone(),
            created_at: user.created_at,
            updated_at: user.updated_at,
        };
        let mut service = crate::basic::ScriptService::new(Arc::clone(&self.0), lib_user);
        service.load_bot_config_params(&self.0, user.bot_id);

        // Inject trigger context variables from UserSession.context_data
        if let Some(obj) = user.context_data.as_object() {
            for key in &["trigger_record_id", "trigger_old_status", "trigger_new_status", "channel"] {
                if let Some(val) = obj.get(*key).and_then(|v| v.as_str()) {
                    let rhai_key = key.to_uppercase();
                    let _ = service.set_variable(&rhai_key, val);
                }
            }
        }

        let result = service.run(script);
        match result {
            Ok(dynamic) => Ok(dynamic.to_string()),
            Err(e) => Err(format!("Script execution error: {}", e)),
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl botlib::traits::ScriptRunner for crate::basic::ScriptService {
    fn run_script(
        &self,
        script: &str,
        _session_id: uuid::Uuid,
        _bot_id: &str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send>> {
        log::info!("ScriptRunner::run_script called: {}", script);
        let script = script.to_string();
        Box::pin(async move { Ok(script) })
    }

    fn get_suggestions(
        &self,
        _session_id: &uuid::Uuid,
        _bot_id: &str,
    ) -> Result<Vec<botlib::models::Suggestion>, String> {
        Ok(Vec::new())
    }
}
