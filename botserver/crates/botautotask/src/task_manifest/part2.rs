use super::*;

impl ItemGroup {
    pub fn new(items: Vec<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            items,
            status: ItemStatus::Pending,
            duration_seconds: None,
            started_at: None,
            completed_at: None,
        }
    }

    pub fn display_name(&self) -> String {
        self.items.join(", ")
    }

    pub fn start(&mut self) {
        self.status = ItemStatus::Running;
        self.started_at = Some(Utc::now());
    }

    pub fn complete(&mut self) {
        self.status = ItemStatus::Completed;
        self.completed_at = Some(Utc::now());
        if let Some(started) = self.started_at {
            self.duration_seconds = Some((Utc::now() - started).num_seconds() as u64);
        }
    }
}
