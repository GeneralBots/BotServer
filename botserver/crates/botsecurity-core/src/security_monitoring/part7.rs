use super::*;

impl SecurityAlert {
    pub fn new(severity: AlertSeverity, title: &str, description: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            severity,
            title: title.to_string(),
            description: description.to_string(),
            event_ids: Vec::new(),
            user_id: None,
            ip_address: None,
            acknowledged: false,
            acknowledged_by: None,
            acknowledged_at: None,
            resolved: false,
            resolved_at: None,
        }
    }

    pub fn with_event(mut self, event_id: Uuid) -> Self {
        self.event_ids.push(event_id);
        self
    }

    pub fn with_events(mut self, event_ids: Vec<Uuid>) -> Self {
        self.event_ids.extend(event_ids);
        self
    }

    pub fn with_user(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_ip(mut self, ip: String) -> Self {
        self.ip_address = Some(ip);
        self
    }

    pub fn acknowledge(&mut self, by: Uuid) {
        self.acknowledged = true;
        self.acknowledged_by = Some(by);
        self.acknowledged_at = Some(Utc::now());
    }

    pub fn resolve(&mut self) {
        self.resolved = true;
        self.resolved_at = Some(Utc::now());
    }
}
