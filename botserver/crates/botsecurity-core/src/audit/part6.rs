use super::*;

impl AuditResource {
    pub fn new(resource_type: &str) -> Self {
        Self {
            resource_type: resource_type.to_string(),
            resource_id: None,
            resource_name: None,
            parent_resource: None,
        }
    }

    pub fn with_id(mut self, id: &str) -> Self {
        self.resource_id = Some(id.to_string());
        self
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.resource_name = Some(name.to_string());
        self
    }

    pub fn with_parent(mut self, parent: AuditResource) -> Self {
        self.parent_resource = Some(Box::new(parent));
        self
    }
}
