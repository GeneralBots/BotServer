use super::*;

impl SopLogger {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            max_entries: 100_000,
            retention_days: 365,
            webhook_url: None,
            siem_endpoint: None,
        }
    }

    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }

    pub fn with_retention_days(mut self, days: i64) -> Self {
        self.retention_days = days;
        self
    }

    pub fn with_webhook(mut self, url: String) -> Self {
        self.webhook_url = Some(url);
        self
    }

    pub fn with_siem_endpoint(mut self, endpoint: String) -> Self {
        self.siem_endpoint = Some(endpoint);
        self
    }

    pub async fn log(&self, entry: SopLogEntry) {
        let entry_clone = entry.clone();

        {
            let mut entries = self.entries.write().await;

            if entries.len() >= self.max_entries {
                let remove_count = self.max_entries / 10;
                entries.drain(0..remove_count);
            }

            entries.push(entry);
        }

        self.log_to_persistent_storage(&entry_clone).await;

        if self.should_alert(&entry_clone) {
            self.send_alert(&entry_clone).await;
        }

        if let Some(ref webhook_url) = self.webhook_url {
            self.send_to_webhook(webhook_url, &entry_clone).await;
        }

        if let Some(ref siem_endpoint) = self.siem_endpoint {
            self.send_to_siem(siem_endpoint, &entry_clone).await;
        }
    }

    pub async fn log_authentication(
        &self,
        user_id: Option<Uuid>,
        email: Option<String>,
        outcome: SopOutcome,
        method: &str,
        ip: Option<String>,
    ) {
        let severity = match outcome {
            SopOutcome::Success => SopSeverity::Info,
            SopOutcome::Failure | SopOutcome::Denied => SopSeverity::Medium,
            _ => SopSeverity::Low,
        };

        let mut entry = SopLogEntry::new(
            SopCategory::Authentication,
            "user_authentication",
            &format!("User authentication attempt via {method}"),
        )
        .with_severity(severity)
        .with_outcome(outcome)
        .with_client_info(ip, None)
        .with_compliance_controls(vec!["CC6.1", "CC6.6"]);

        if let Some(uid) = user_id {
            entry = entry.with_user(uid, email);
        }

        self.log(entry).await;
    }

    pub async fn log_authorization(
        &self,
        user_id: Uuid,
        resource_type: &str,
        resource_id: &str,
        action: &str,
        outcome: SopOutcome,
    ) {
        let severity = match outcome {
            SopOutcome::Denied => SopSeverity::Medium,
            _ => SopSeverity::Info,
        };

        let entry = SopLogEntry::new(
            SopCategory::Authorization,
            "authorization_check",
            &format!("Authorization check for {action} on {resource_type}"),
        )
        .with_severity(severity)
        .with_outcome(outcome)
        .with_user(user_id, None)
        .with_resource(resource_type, resource_id)
        .with_metadata("action", serde_json::json!(action))
        .with_compliance_controls(vec!["CC6.1", "CC6.3"]);

        self.log(entry).await;
    }

    pub async fn log_data_access(
        &self,
        user_id: Uuid,
        organization_id: Uuid,
        resource_type: &str,
        resource_id: &str,
        access_type: &str,
    ) {
        let entry = SopLogEntry::new(
            SopCategory::DataAccess,
            "data_access",
            &format!("{access_type} access to {resource_type}"),
        )
        .with_severity(SopSeverity::Info)
        .with_outcome(SopOutcome::Success)
        .with_user(user_id, None)
        .with_organization(organization_id)
        .with_resource(resource_type, resource_id)
        .with_metadata("access_type", serde_json::json!(access_type))
        .with_compliance_controls(vec!["CC6.1", "CC6.5", "PI1.1"]);

        self.log(entry).await;
    }

    pub async fn log_data_modification(
        &self,
        user_id: Uuid,
        organization_id: Uuid,
        resource_type: &str,
        resource_id: &str,
        modification_type: &str,
        changes: Option<serde_json::Value>,
    ) {
        let mut entry = SopLogEntry::new(
            SopCategory::DataModification,
            "data_modification",
            &format!("{modification_type} on {resource_type}"),
        )
        .with_severity(SopSeverity::Medium)
        .with_outcome(SopOutcome::Success)
        .with_user(user_id, None)
        .with_organization(organization_id)
        .with_resource(resource_type, resource_id)
        .with_metadata("modification_type", serde_json::json!(modification_type))
        .with_compliance_controls(vec!["CC6.1", "CC7.2", "PI1.4"]);

        if let Some(c) = changes {
            entry = entry.with_metadata("changes", c);
        }

        self.log(entry).await;
    }

    pub async fn log_configuration_change(
        &self,
        user_id: Uuid,
        config_type: &str,
        config_key: &str,
        old_value: Option<&str>,
        new_value: Option<&str>,
    ) {
        let mut entry = SopLogEntry::new(
            SopCategory::ConfigurationChange,
            "configuration_change",
            &format!("Configuration change: {config_type}/{config_key}"),
        )
        .with_severity(SopSeverity::High)
        .with_outcome(SopOutcome::Success)
        .with_user(user_id, None)
        .with_resource(config_type, config_key)
        .with_compliance_controls(vec!["CC6.1", "CC7.1", "CC8.1"]);

        if let Some(old) = old_value {
            entry = entry.with_metadata("old_value", serde_json::json!(old));
        }
        if let Some(new) = new_value {
            entry = entry.with_metadata("new_value", serde_json::json!(new));
        }

        self.log(entry).await;
    }

    pub async fn log_security_event(
        &self,
        event_type: &str,
        description: &str,
        severity: SopSeverity,
        metadata: HashMap<String, serde_json::Value>,
    ) {
        let mut entry = SopLogEntry::new(SopCategory::SecurityEvent, event_type, description)
            .with_severity(severity)
            .with_outcome(SopOutcome::Success)
            .with_compliance_controls(vec!["CC7.2", "CC7.3", "CC7.4"]);

        for (key, value) in metadata {
            entry = entry.with_metadata(&key, value);
        }

        self.log(entry).await;
    }

    pub async fn log_incident_response(
        &self,
        incident_id: &str,
        action: &str,
        responder_id: Uuid,
        details: &str,
    ) {
        let entry = SopLogEntry::new(
            SopCategory::IncidentResponse,
            action,
            &format!("Incident response action: {details}"),
        )
        .with_severity(SopSeverity::High)
        .with_outcome(SopOutcome::Success)
        .with_user(responder_id, None)
        .with_resource("incident", incident_id)
        .with_compliance_controls(vec!["CC7.3", "CC7.4", "CC7.5"]);

        self.log(entry).await;
    }

    pub async fn log_backup_operation(
        &self,
        operation: &str,
        backup_id: &str,
        outcome: SopOutcome,
        size_bytes: Option<u64>,
    ) {
        let mut entry = SopLogEntry::new(
            SopCategory::BackupOperation,
            operation,
            &format!("Backup operation: {operation}"),
        )
        .with_severity(SopSeverity::Medium)
        .with_outcome(outcome)
        .with_resource("backup", backup_id)
        .with_compliance_controls(vec!["A1.2", "A1.3"]);

        if let Some(size) = size_bytes {
            entry = entry.with_metadata("size_bytes", serde_json::json!(size));
        }

        self.log(entry).await;
    }
}
