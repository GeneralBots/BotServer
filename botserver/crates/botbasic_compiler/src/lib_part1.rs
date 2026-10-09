use super::*;

impl fmt::Debug for CompilerCallbacks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompilerCallbacks")
            .field("execute_set_schedule", &self.execute_set_schedule.is_some())
            .field("execute_webhook", &self.execute_webhook.is_some())
            .field("execute_use_website", &self.execute_use_website.is_some())
            .field("process_table_definitions", &self.process_table_definitions.is_some())
            .field("create_runtime", &self.create_runtime.is_some())
            .field("execute_on_update", &self.execute_on_update.is_some())
            .finish()
    }
}
impl Default for CompilerCallbacks {
    fn default() -> Self {
        Self::new()
    }
}
impl CompilerCallbacks {
    pub fn new() -> Self {
        Self {
            execute_set_schedule: None,
            execute_webhook: None,
            execute_use_website: None,
            process_table_definitions: None,
            create_runtime: None,
            execute_on_update: None,
            execute_on_event: None,
        }
    }
}
