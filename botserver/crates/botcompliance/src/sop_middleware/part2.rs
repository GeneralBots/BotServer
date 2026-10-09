use super::*;

impl Default for SopSearchQuery {
    fn default() -> Self {
        Self {
            category: None,
            severity: None,
            outcome: None,
            user_id: None,
            organization_id: None,
            resource_type: None,
            from_date: None,
            to_date: None,
            operation_contains: None,
            compliance_control: None,
            page: 1,
            per_page: 50,
        }
    }
}
