use super::*;

impl Default for EpisodicMemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold: 4,
            history: 2,
            model: "fast".to_string(),
            max_episodes: 100,
            retention_days: 365,
            auto_summarize: true,
        }
    }
}
