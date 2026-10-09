use super::*;

impl TaskProgressEvent {
    pub fn new(
        task_id: impl Into<String>,
        step: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            event_type: "task_progress".to_string(),
            task_id: task_id.into(),
            step: step.into(),
            message: message.into(),
            progress: 0,
            total_steps: 0,
            current_step: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
            details: None,
            error: None,
            activity: None,
            text: None,
        }
    }

    #[must_use]
    pub fn with_progress(mut self, current: u8, total: u8) -> Self {
        self.current_step = current;
        self.total_steps = total;
        self.progress = if total > 0 {
            ((current as u16 * 100) / total as u16) as u8
        } else {
            0
        };
        self
    }

    #[must_use]
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    #[must_use]
    pub fn with_activity(mut self, activity: AgentActivity) -> Self {
        self.activity = Some(activity);
        self
    }

    #[must_use]
    pub fn with_event_type(mut self, event_type: impl Into<String>) -> Self {
        self.event_type = event_type.into();
        self
    }

    #[must_use]
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.event_type = "task_error".to_string();
        self.error = Some(error.into());
        self
    }

    #[must_use]
    pub fn completed(mut self) -> Self {
        self.event_type = "task_completed".to_string();
        self.progress = 100;
        self
    }

    pub fn started(
        task_id: impl Into<String>,
        message: impl Into<String>,
        total_steps: u8,
    ) -> Self {
        Self {
            event_type: "task_started".to_string(),
            task_id: task_id.into(),
            step: "init".to_string(),
            message: message.into(),
            progress: 0,
            total_steps,
            current_step: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
            details: None,
            error: None,
            activity: None,
            text: None,
        }
    }
}
