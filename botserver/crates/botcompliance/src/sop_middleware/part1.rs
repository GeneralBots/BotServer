use super::*;

impl SopLogEntry {
    pub fn new(category: SopCategory, operation: &str, description: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            category,
            severity: SopSeverity::Info,
            outcome: SopOutcome::Pending,
            operation: operation.to_string(),
            description: description.to_string(),
            user_id: None,
            user_email: None,
            organization_id: None,
            resource_type: None,
            resource_id: None,
            ip_address: None,
            user_agent: None,
            request_method: None,
            request_path: None,
            request_id: None,
            response_status: None,
            duration_ms: None,
            metadata: HashMap::new(),
            compliance_controls: Vec::new(),
            evidence_reference: None,
        }
    }

    pub fn with_severity(mut self, severity: SopSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_outcome(mut self, outcome: SopOutcome) -> Self {
        self.outcome = outcome;
        self
    }

    pub fn with_user(mut self, user_id: Uuid, email: Option<String>) -> Self {
        self.user_id = Some(user_id);
        self.user_email = email;
        self
    }

    pub fn with_organization(mut self, org_id: Uuid) -> Self {
        self.organization_id = Some(org_id);
        self
    }

    pub fn with_resource(mut self, resource_type: &str, resource_id: &str) -> Self {
        self.resource_type = Some(resource_type.to_string());
        self.resource_id = Some(resource_id.to_string());
        self
    }

    pub fn with_request_info(
        mut self,
        method: &str,
        path: &str,
        request_id: Option<String>,
    ) -> Self {
        self.request_method = Some(method.to_string());
        self.request_path = Some(path.to_string());
        self.request_id = request_id;
        self
    }

    pub fn with_client_info(mut self, ip: Option<String>, user_agent: Option<String>) -> Self {
        self.ip_address = ip;
        self.user_agent = user_agent;
        self
    }

    pub fn with_response(mut self, status: u16, duration_ms: u64) -> Self {
        self.response_status = Some(status);
        self.duration_ms = Some(duration_ms);
        self
    }

    pub fn with_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }

    pub fn with_compliance_controls(mut self, controls: Vec<&str>) -> Self {
        self.compliance_controls = controls.into_iter().map(String::from).collect();
        self
    }

    pub fn with_evidence_reference(mut self, reference: &str) -> Self {
        self.evidence_reference = Some(reference.to_string());
        self
    }
}
