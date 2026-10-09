use super::*;

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            drive: self.drive.clone(),
            #[cfg(feature = "cache")]
            cache: self.cache.clone(),
            bucket_name: self.bucket_name.clone(),
            config: self.config.clone(),
            conn: self.conn.clone(),
            database_url: self.database_url.clone(),
            bot_database_manager: Arc::clone(&self.bot_database_manager),
            session_manager: Arc::clone(&self.session_manager),
            metrics_collector: self.metrics_collector.clone(),
            channels: Arc::clone(&self.channels),
            response_channels: Arc::clone(&self.response_channels),
            active_streams: Arc::clone(&self.active_streams),
            hear_channels: Arc::clone(&self.hear_channels),
            web_adapter: Arc::clone(&self.web_adapter),
            voice_adapter: Arc::clone(&self.voice_adapter),
            kb_manager: self.kb_manager.clone(),
            script_runner: self.script_runner.clone(),
            task_engine: self.task_engine.clone(),
            task_scheduler: self.task_scheduler.clone(),
            extensions: self.extensions.clone(),
            attendant_broadcast: self.attendant_broadcast.clone(),
            task_progress_broadcast: self.task_progress_broadcast.clone(),
            billing_alert_broadcast: self.billing_alert_broadcast.clone(),
            task_manifests: Arc::clone(&self.task_manifests),
            terminal_manager: self.terminal_manager.clone(),
            project_service: self.project_service.clone(),
            legal_service: self.legal_service.clone(),
            jwt_manager: self.jwt_manager.clone(),
            auth_provider_registry: self.auth_provider_registry.clone(),
            rbac_manager: self.rbac_manager.clone(),
            auth_service: self.auth_service.clone(),
            llm_provider: self.llm_provider.clone(),
            dynamic_llm_provider: self.dynamic_llm_provider.clone(),
            start_bas_guards: Arc::clone(&self.start_bas_guards),
            pending_stream_responses: Arc::clone(&self.pending_stream_responses),
        }
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("bucket_name", &self.bucket_name)
            .field("config", &self.config.is_some())
            .field("conn", &"DbPool")
            .field("database_url", &"[REDACTED]")
            .field("bot_database_manager", &"Arc<dyn BotDatabaseService>")
            .field("session_manager", &"Arc<Mutex<dyn SessionManagerService>>")
            .field("extensions", &self.extensions)
            .field("attendant_broadcast", &self.attendant_broadcast.is_some())
            .field("task_progress_broadcast", &self.task_progress_broadcast.is_some())
            .field("jwt_manager", &self.jwt_manager.is_some())
            .field("auth_provider_registry", &self.auth_provider_registry.is_some())
            .field("rbac_manager", &self.rbac_manager.is_some())
            .finish()
    }
}

impl AppState {
    pub fn broadcast_task_progress(&self, event: TaskProgressEvent) {
        log::info!(
            "Broadcasting: task_id={}, step={}, message={}",
            event.task_id, event.step, event.message
        );
        if let Some(tx) = &self.task_progress_broadcast {
            let receiver_count = tx.receiver_count();
            log::info!("Broadcast channel has {} receivers", receiver_count);
            match tx.send(event) {
                Ok(_) => { log::info!("Event sent successfully"); }
                Err(e) => { log::warn!("No listeners for task progress: {e}"); }
            }
        } else {
            log::warn!("No broadcast channel configured!");
        }
    }

    pub fn emit_progress(&self, task_id: &str, step: &str, message: &str, current: u8, total: u8) {
        let event = TaskProgressEvent::new(task_id, step, message).with_progress(current, total);
        self.broadcast_task_progress(event);
    }

    pub fn emit_progress_with_details(
        &self, task_id: &str, step: &str, message: &str, current: u8, total: u8, details: &str,
    ) {
        let event = TaskProgressEvent::new(task_id, step, message)
            .with_progress(current, total)
            .with_details(details);
        self.broadcast_task_progress(event);
    }

    pub fn emit_activity(
        &self, task_id: &str, step: &str, message: &str, current: u8, total: u8, activity: AgentActivity,
    ) {
        let event = TaskProgressEvent::new(task_id, step, message)
            .with_progress(current, total)
            .with_activity(activity);
        self.broadcast_task_progress(event);
    }

    pub fn emit_task_started(&self, task_id: &str, message: &str, total_steps: u8) {
        let event = TaskProgressEvent::started(task_id, message, total_steps);
        self.broadcast_task_progress(event);
    }

    pub fn emit_task_completed(&self, task_id: &str, message: &str) {
        let event = TaskProgressEvent::new(task_id, "complete", message).completed();
        self.broadcast_task_progress(event);
    }

    pub fn emit_task_error(&self, task_id: &str, step: &str, error: &str) {
        let event = TaskProgressEvent::new(task_id, step, "Task failed").with_error(error);
        self.broadcast_task_progress(event);
    }

    pub fn emit_llm_stream(&self, task_id: &str, text: &str) {
        let event = TaskProgressEvent::llm_stream(task_id, text);
        if let Some(tx) = &self.task_progress_broadcast {
            let _ = tx.send(event);
        }
    }
}
