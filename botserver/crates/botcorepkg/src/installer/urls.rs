use super::*;

pub(super) fn get_thirdparty_config() -> &'static ThirdPartyConfig {
    THIRDPARTY_CONFIG.get_or_init(|| {
        let toml_str = include_str!("../../../../3rdparty.toml");
        match toml::from_str::<ThirdPartyConfig>(toml_str) {
            Ok(config) => config,
            Err(e) => {
                log::error!("CRITICAL: Failed to parse embedded 3rdparty.toml: {e}");
                ThirdPartyConfig {
                    components: HashMap::new(),
                    models: HashMap::new(),
                }
            }
        }
    })
}

pub fn get_component_url(name: &str) -> Option<String> {
    get_thirdparty_config()
        .components
        .get(name)
        .map(|c| c.effective_url().to_string())
}

pub fn get_model_url(name: &str) -> Option<String> {
    get_thirdparty_config()
        .models
        .get(name)
        .map(|c| c.effective_url().to_string())
}
