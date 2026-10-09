use super::*;

impl ImageGeneratorConfig {
    pub fn from_provider(provider: &dyn ConfigProvider, bot_id: &Uuid) -> Self {
        Self {
            model: provider
                .get_config(bot_id, "image-generator-model", None)
                .unwrap_or_default(),
            steps: provider
                .get_config(bot_id, "image-generator-steps", Some("4"))
                .unwrap_or( "4".to_string())
                .parse()
                .unwrap_or(4),
            width: provider
                .get_config(bot_id, "image-generator-width", Some("512"))
                .unwrap_or( "512".to_string())
                .parse()
                .unwrap_or(512),
            height: provider
                .get_config(bot_id, "image-generator-height", Some("512"))
                .unwrap_or( "512".to_string())
                .parse()
                .unwrap_or(512),
            gpu_layers: provider
                .get_config(bot_id, "image-generator-gpu-layers", Some("20"))
                .unwrap_or( "20".to_string())
                .parse()
                .unwrap_or(20),
            batch_size: provider
                .get_config(bot_id, "image-generator-batch-size", Some("1"))
                .unwrap_or( "1".to_string())
                .parse()
                .unwrap_or(1),
        }
    }
}
