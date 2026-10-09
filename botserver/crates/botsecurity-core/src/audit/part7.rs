use super::*;

impl AuditEvent {
    pub fn new(event_type: AuditEventType, actor: AuditActor) -> Self {
        let id = Uuid::new_v4();
        let timestamp = Utc::now();
        let category = event_type.category();
        let severity = event_type.severity();
        let action = event_type.as_str().to_string();

        let mut event = Self {
            id,
            timestamp,
            event_type,
            category,
            severity,
            outcome: AuditOutcome::Success,
            actor,
            resource: None,
            action,
            description: String::new(),
            metadata: HashMap::new(),
            request_id: None,
            organization_id: None,
            previous_hash: None,
            hash: String::new(),
        };

        event.hash = event.compute_hash();
        event
    }

    pub fn with_outcome(mut self, outcome: AuditOutcome) -> Self {
        self.outcome = outcome;
        self.hash = self.compute_hash();
        self
    }

    pub fn with_resource(mut self, resource: AuditResource) -> Self {
        self.resource = Some(resource);
        self.hash = self.compute_hash();
        self
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self.hash = self.compute_hash();
        self
    }

    pub fn with_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self.hash = self.compute_hash();
        self
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self.hash = self.compute_hash();
        self
    }

    pub fn with_organization(mut self, org_id: Uuid) -> Self {
        self.organization_id = Some(org_id);
        self.hash = self.compute_hash();
        self
    }

    pub fn with_previous_hash(mut self, hash: String) -> Self {
        self.previous_hash = Some(hash);
        self.hash = self.compute_hash();
        self
    }

    pub(super) fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();

        hasher.update(self.id.as_bytes());
        hasher.update(self.timestamp.to_rfc3339().as_bytes());
        hasher.update(self.event_type.as_str().as_bytes());
        hasher.update(self.outcome.as_str().as_bytes());
        hasher.update(self.action.as_bytes());
        hasher.update(self.description.as_bytes());

        if let Some(ref prev) = self.previous_hash {
            hasher.update(prev.as_bytes());
        }

        if let Some(ref actor_id) = self.actor.user_id {
            hasher.update(actor_id.as_bytes());
        }

        let result = hasher.finalize();
        hex::encode(result)
    }

    pub fn verify_hash(&self) -> bool {
        self.hash == self.compute_hash()
    }

    pub fn is_security_event(&self) -> bool {
        self.category == AuditEventCategory::Security
    }

    pub fn is_critical(&self) -> bool {
        self.severity == AuditSeverity::Critical
    }
}
