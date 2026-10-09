use super::*;

impl VideoGeneratorConfig {
    pub fn from_provider(provider: &dyn ConfigProvider, bot_id: &Uuid) -> Self {
        Self {
            model: provider
                .get_config(bot_id, "video-generator-model", None)
                .unwrap_or_default(),
            frames: provider
                .get_config(bot_id, "video-generator-frames", Some("24"))
                .unwrap_or( "24".to_string())
                .parse()
                .unwrap_or(24),
            fps: provider
                .get_config(bot_id, "video-generator-fps", Some("8"))
                .unwrap_or( "8".to_string())
                .parse()
                .unwrap_or(8),
            width: provider
                .get_config(bot_id, "video-generator-width", Some("320"))
                .unwrap_or( "320".to_string())
                .parse()
                .unwrap_or(320),
            height: provider
                .get_config(bot_id, "video-generator-height", Some("576"))
                .unwrap_or( "576".to_string())
                .parse()
                .unwrap_or(576),
            gpu_layers: provider
                .get_config(bot_id, "video-generator-gpu-layers", Some("15"))
                .unwrap_or( "15".to_string())
                .parse()
                .unwrap_or(15),
            batch_size: provider
                .get_config(bot_id, "video-generator-batch-size", Some("1"))
                .unwrap_or( "1".to_string())
                .parse()
                .unwrap_or(1),
        }
    }
}
