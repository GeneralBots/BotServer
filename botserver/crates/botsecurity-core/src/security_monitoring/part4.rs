use super::*;

impl SecurityEvent {
    pub fn new(event_type: SecurityEventType) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type,
            severity: event_type.severity(),
            user_id: None,
            ip_address: None,
            user_agent: None,
            location: None,
            device_fingerprint: None,
            details: HashMap::new(),
            request_id: None,
        }
    }

    pub fn with_user(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_ip(mut self, ip: String) -> Self {
        self.ip_address = Some(ip);
        self
    }

    pub fn with_user_agent(mut self, ua: String) -> Self {
        self.user_agent = Some(ua);
        self
    }

    pub fn with_location(mut self, location: GeoLocation) -> Self {
        self.location = Some(location);
        self
    }

    pub fn with_device(mut self, fingerprint: String) -> Self {
        self.device_fingerprint = Some(fingerprint);
        self
    }

    pub fn with_detail(mut self, key: &str, value: serde_json::Value) -> Self {
        self.details.insert(key.to_string(), value);
        self
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    pub fn is_critical(&self) -> bool {
        self.severity == AlertSeverity::Critical
    }
}
