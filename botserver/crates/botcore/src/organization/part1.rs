use super::*;

impl Organization {
    pub fn new(name: String, owner_id: Uuid) -> Self {
        let slug = slugify(&name);
        let now = Utc::now();

        Self {
            id: Uuid::new_v4(),
            name,
            slug,
            description: None,
            logo_url: None,
            website: None,
            plan_id: "free".to_string(),
            owner_id,
            settings: OrganizationSettings::default(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    pub fn with_plan(mut self, plan_id: String) -> Self {
        self.plan_id = plan_id;
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
}
