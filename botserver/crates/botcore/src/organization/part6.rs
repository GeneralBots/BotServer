use super::*;

impl GroupMember {
    pub fn new(group_id: Uuid, user_id: Uuid, added_by: Option<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            group_id,
            user_id,
            added_by,
            created_at: Utc::now(),
        }
    }
}
/// Default groups for an organization
pub fn default_groups(organization_id: Uuid) -> Vec<Group> {
    vec![
        Group::new(organization_id, "everyone", "Everyone")
            .with_description("All members of the organization".to_string())
            .with_permissions(vec!["basic:access".to_string(), "kb:read:public".to_string()])
            .as_system(),
        Group::new(organization_id, "admins", "Administrators")
            .with_description("Full organization administrators".to_string())
            .with_permissions(vec![
                "org:manage".to_string(),
                "org:members".to_string(),
                "org:settings".to_string(),
                "org:billing".to_string(),
                "bot:*".to_string(),
                "kb:*".to_string(),
                "app:*".to_string(),
                "analytics:*".to_string(),
            ])
            .as_system(),
        Group::new(organization_id, "managers", "Managers")
            .with_description("Department and team managers".to_string())
            .with_permissions(vec![
                "org:members:view".to_string(),
                "bot:create".to_string(),
                "bot:edit".to_string(),
                "bot:delete".to_string(),
                "kb:read".to_string(),
                "kb:write".to_string(),
                "app:create".to_string(),
                "app:edit".to_string(),
                "analytics:view".to_string(),
                "analytics:export".to_string(),
            ])
            .as_system(),
        Group::new(organization_id, "developers", "Developers")
            .with_description("Development team members".to_string())
            .with_permissions(vec![
                "bot:create".to_string(),
                "bot:edit".to_string(),
                "kb:write".to_string(),
                "app:create".to_string(),
            ]),
        Group::new(organization_id, "human_resources", "Human Resources")
            .with_description("HR team members".to_string())
            .with_permissions(vec![
                "people:manage".to_string(),
                "kb:read".to_string(),
                "org:members:view".to_string(),
                "analytics:view".to_string(),
            ]),
        Group::new(organization_id, "finance", "Finance")
            .with_description("Finance department".to_string())
            .with_permissions(vec![
                "billing:view".to_string(),
                "analytics:view".to_string(),
                "kb:read".to_string(),
            ]),
        Group::new(organization_id, "marketing", "Marketing")
            .with_description("Marketing department".to_string())
            .with_permissions(vec![
                "campaigns:manage".to_string(),
                "social:post".to_string(),
                "analytics:view".to_string(),
                "kb:read".to_string(),
                "kb:write".to_string(),
            ]),
        Group::new(organization_id, "support", "Support Team")
            .with_description("Customer support team".to_string())
            .with_permissions(vec![
                "bot:view".to_string(),
                "kb:read".to_string(),
                "analytics:view".to_string(),
                "tickets:manage".to_string(),
            ]),
        Group::new(organization_id, "content_managers", "Content Managers")
            .with_description("Content management team".to_string())
            .with_permissions(vec![
                "kb:write".to_string(),
                "kb:admin".to_string(),
                "docs:manage".to_string(),
            ]),
        Group::new(organization_id, "sales", "Sales")
            .with_description("Sales department".to_string())
            .with_permissions(vec![
                "crm:view".to_string(),
                "crm:manage".to_string(),
                "analytics:view".to_string(),
                "kb:read".to_string(),
            ]),
        Group::new(organization_id, "viewers", "Viewers (Read-Only)")
            .with_description("Read-only access for external viewers".to_string())
            .with_permissions(vec![
                "bot:view".to_string(),
                "kb:read".to_string(),
                "app:view".to_string(),
                "analytics:view".to_string(),
            ])
            .as_system(),
        Group::new(organization_id, "integration_services", "Integration Services")
            .with_description("Service accounts for integrations".to_string())
            .with_permissions(vec![
                "webhooks:manage".to_string(),
                "api:access".to_string(),
                "sources:connect".to_string(),
            ])
            .as_system(),
    ]
}

// ============================================================================
// User Role Assignment
// ============================================================================
