use super::*;

impl BotModelsConfig {
    pub fn from_provider(provider: &dyn ConfigProvider, bot_id: &Uuid) -> Self {
        let enabled = provider
            .get_config(bot_id, "botmodels-enabled", Some("false"))
            .unwrap_or_default()
            .to_lowercase()
            == "true";

        let host = provider
            .get_config(bot_id, "botmodels-host", Some("0.0.0.0"))
            .unwrap_or("0.0.0.0".to_string());

        let port = provider
            .get_config(bot_id, "botmodels-port", Some("8085"))
            .unwrap_or("8085".to_string())
            .parse()
            .unwrap_or(8085);

        let api_key = provider
            .get_config(bot_id, "botmodels-api-key", Some(""))
            .unwrap_or_default();

        let use_https = provider
            .get_config(bot_id, "botmodels-https", Some("false"))
            .unwrap_or_default()
            .to_lowercase()
            == "true";

        Self {
            enabled,
            host,
            port,
            api_key,
            use_https,
        }
    }

    pub fn base_url(&self) -> String {
        let protocol = if self.use_https { "https" } else { "http" };
        format!("{}://{}:{}", protocol, self.host, self.port)
    }
}
