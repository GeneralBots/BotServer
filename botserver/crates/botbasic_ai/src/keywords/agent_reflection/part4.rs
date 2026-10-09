use super::*;

impl std::fmt::Debug for ReflectionEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReflectionEngine")
            .field("config", &self.config)
            .field("bot_id", &self.bot_id)
            .finish_non_exhaustive()
    }
}
