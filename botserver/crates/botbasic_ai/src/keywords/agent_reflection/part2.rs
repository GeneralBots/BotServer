use super::*;

impl Default for ReflectionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval: 10,
            custom_prompt: None,
            auto_apply: false,
            improvement_threshold: 6.0,
            max_insights: 100,
            reflection_types: vec![ReflectionType::ConversationQuality],
        }
    }
}
impl ReflectionConfig {
    pub fn from_bot_config(_state: &Arc<dyn BasicRuntime>, bot_id: Uuid) -> Self {
        let mut config = Self::default();

        if let Some(vault_configs) = read_bot_config_from_vault(&bot_id) {
            for (config_key, config_value) in vault_configs {
                match config_key.as_str() {
                    "bot-reflection-enabled" => {
                        config.enabled = config_value.to_lowercase() == "true";
                    }
                    "bot-reflection-interval" => {
                        config.interval = config_value.parse().unwrap_or(10);
                    }
                    "bot-reflection-prompt" => {
                        config.custom_prompt = Some(config_value);
                    }
                    "bot-improvement-auto-apply" => {
                        config.auto_apply = config_value.to_lowercase() == "true";
                    }
                    "bot-improvement-threshold" => {
                        config.improvement_threshold = config_value.parse().unwrap_or(6.0);
                    }
                    "bot-reflection-types" => {
                        config.reflection_types = config_value
                            .split(';')
                            .map(|s| ReflectionType::from(s.trim()))
                            .collect();
                    }
                    _ => {}
                }
            }
        }

        config
    }
}
