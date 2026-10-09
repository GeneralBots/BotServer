use super::*;

impl std::fmt::Debug for ApiToolGenerator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiToolGenerator")
            .field("bot_id", &self.bot_id)
            .field("work_path", &self.work_path)
            .finish_non_exhaustive()
    }
}
