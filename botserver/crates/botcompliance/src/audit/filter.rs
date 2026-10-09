use super::*;

impl AuditFilter {
    pub(super) fn matches(&self, event: &AuditEvent) -> bool {
        if let Some(types) = &self.event_types {
            if !types.contains(&event.event_type) {
                return false;
            }
        }

        if let Some(severity) = &self.severity {
            if event.severity < *severity {
                return false;
            }
        }

        if let Some(user_id) = &self.user_id {
            if event.user_id != Some(*user_id) {
                return false;
            }
        }

        if let Some(since) = &self.since {
            if event.timestamp < *since {
                return false;
            }
        }

        if let Some(until) = &self.until {
            if event.timestamp > *until {
                return false;
            }
        }

        if let Some(outcome) = &self.outcome {
            if event.outcome != *outcome {
                return false;
            }
        }

        true
    }
}
