use super::*;

impl Default for AuditQuery {
    fn default() -> Self {
        Self {
            start_time: None,
            end_time: None,
            event_types: None,
            categories: None,
            severities: None,
            outcomes: None,
            actor_id: None,
            resource_type: None,
            resource_id: None,
            organization_id: None,
            request_id: None,
            search_text: None,
            limit: 100,
            offset: 0,
        }
    }
}
impl AuditQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.start_time = Some(start);
        self.end_time = Some(end);
        self
    }

    pub fn with_event_types(mut self, types: Vec<AuditEventType>) -> Self {
        self.event_types = Some(types);
        self
    }

    pub fn with_categories(mut self, categories: Vec<AuditEventCategory>) -> Self {
        self.categories = Some(categories);
        self
    }

    pub fn with_actor(mut self, actor_id: Uuid) -> Self {
        self.actor_id = Some(actor_id);
        self
    }

    pub fn with_resource(mut self, resource_type: &str, resource_id: &str) -> Self {
        self.resource_type = Some(resource_type.to_string());
        self.resource_id = Some(resource_id.to_string());
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }
}
