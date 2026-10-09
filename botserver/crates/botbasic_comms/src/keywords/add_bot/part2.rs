use super::*;

impl BotTrigger {
    #[must_use]
    pub fn from_keywords(keywords: Vec<String>) -> Self {
        Self {
            trigger_type: TriggerType::Keyword,
            keywords: Some(keywords),
            tools: None,
            schedule: None,
            event_name: None,
        }
    }

    #[must_use]
    pub fn from_tools(tools: Vec<String>) -> Self {
        Self {
            trigger_type: TriggerType::Tool,
            keywords: None,
            tools: Some(tools),
            schedule: None,
            event_name: None,
        }
    }

    #[must_use]
    pub fn from_schedule(cron: String) -> Self {
        Self {
            trigger_type: TriggerType::Schedule,
            keywords: None,
            tools: None,
            schedule: Some(cron),
            event_name: None,
        }
    }

    #[must_use]
    pub fn always() -> Self {
        Self {
            trigger_type: TriggerType::Always,
            keywords: None,
            tools: None,
            schedule: None,
            event_name: None,
        }
    }
}
