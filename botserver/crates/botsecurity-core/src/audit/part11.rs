use super::*;

impl<S: AuditStore> AuditLogger<S> {
    pub fn new(config: AuditConfig, store: S) -> Self {
        Self {
            config,
            store,
            buffer: Arc::new(RwLock::new(Vec::new())),
            last_hash: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn log(&self, mut event: AuditEvent) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        if event.severity < self.config.min_severity {
            return Ok(());
        }

        if !self.config.categories_enabled.contains(&event.category) {
            return Ok(());
        }

        if self.config.tamper_evident {
            let mut last_hash = self.last_hash.write().await;
            if let Some(ref hash) = *last_hash {
                event = event.with_previous_hash(hash.clone());
            }
            *last_hash = Some(event.hash.clone());
        }

        if event.is_critical() {
            info!(
                "CRITICAL AUDIT: {} - {} - {}",
                event.event_type.as_str(),
                event.action,
                event.description
            );
        }

        if self.config.async_logging {
            let mut buffer = self.buffer.write().await;
            buffer.push(event);

            if buffer.len() >= self.config.buffer_size {
                let events: Vec<AuditEvent> = buffer.drain(..).collect();
                drop(buffer);
                self.store.store_batch(events).await?;
            }
        } else {
            self.store.store(event).await?;
        }

        Ok(())
    }

    pub async fn log_auth_success(&self, actor: AuditActor, method: &str) -> Result<()> {
        let event = AuditEvent::new(AuditEventType::LoginSuccess, actor)
            .with_description(&format!("Successful authentication via {method}"))
            .with_metadata("auth_method", serde_json::json!(method));

        self.log(event).await
    }

    pub async fn log_auth_failure(&self, actor: AuditActor, reason: &str) -> Result<()> {
        let event = AuditEvent::new(AuditEventType::LoginFailure, actor)
            .with_outcome(AuditOutcome::Failure)
            .with_description(&format!("Authentication failed: {reason}"))
            .with_metadata("failure_reason", serde_json::json!(reason));

        self.log(event).await
    }

    pub async fn log_permission_denied(
        &self,
        actor: AuditActor,
        resource: AuditResource,
        permission: &str,
    ) -> Result<()> {
        let event = AuditEvent::new(AuditEventType::PermissionDenied, actor)
            .with_outcome(AuditOutcome::Failure)
            .with_resource(resource)
            .with_description(&format!("Permission denied: {permission}"))
            .with_metadata("required_permission", serde_json::json!(permission));

        self.log(event).await
    }

    pub async fn log_data_access(
        &self,
        actor: AuditActor,
        resource: AuditResource,
        action: &str,
    ) -> Result<()> {
        let event = AuditEvent::new(AuditEventType::DataRead, actor)
            .with_resource(resource)
            .with_description(&format!("Data accessed: {action}"));

        self.log(event).await
    }

    pub async fn log_data_modification(
        &self,
        actor: AuditActor,
        resource: AuditResource,
        event_type: AuditEventType,
        description: &str,
    ) -> Result<()> {
        let event = AuditEvent::new(event_type, actor)
            .with_resource(resource)
            .with_description(description);

        self.log(event).await
    }

    pub async fn log_security_event(
        &self,
        actor: AuditActor,
        event_type: AuditEventType,
        description: &str,
        metadata: HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let mut event = AuditEvent::new(event_type, actor).with_description(description);

        for (key, value) in metadata {
            event = event.with_metadata(&key, value);
        }

        self.log(event).await
    }

    pub async fn flush(&self) -> Result<()> {
        let events: Vec<AuditEvent> = {
            let mut buffer = self.buffer.write().await;
            buffer.drain(..).collect()
        };

        if !events.is_empty() {
            self.store.store_batch(events).await?;
        }

        Ok(())
    }

    pub async fn query(&self, query: AuditQuery) -> Result<AuditQueryResult> {
        self.store.query(query).await
    }

    pub fn config(&self) -> &AuditConfig {
        &self.config
    }
}
pub fn create_audit_logger() -> AuditLogger<InMemoryAuditStore> {
    AuditLogger::new(AuditConfig::default(), InMemoryAuditStore::default())
}
