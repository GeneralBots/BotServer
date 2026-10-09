use super::*;

impl std::fmt::Display for ProductConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "IO error reading .product file: {}", e),
            Self::ParseError(msg) => write!(f, "Parse error in .product file: {}", msg),
            Self::LockError => write!(f, "Failed to acquire lock on product configuration"),
        }
    }
}
impl std::error::Error for ProductConfigError {}
/// Helper function to get product name
pub fn get_product_name() -> String {
    PRODUCT_CONFIG
        .read()
        .map(|c| c.name.clone())
        .unwrap_or_else(|_| "General Bots".to_string())
}
/// Helper function to check whether an app is a preview application (#1348).
/// When the product configuration cannot be read, the answer is `false`: an
/// unreadable configuration must not promote unreleased apps into the launcher.
pub fn is_app_preview(app: &str) -> bool {
    PRODUCT_CONFIG
        .read()
        .map(|c| c.is_app_preview(app))
        .unwrap_or(false)
}
/// Helper function to get default theme
pub fn get_default_theme() -> String {
    PRODUCT_CONFIG
        .read()
        .map(|c| c.theme.clone())
        .unwrap_or_else(|_| "sentient".to_string())
}
/// Helper function to replace branding in text
pub fn replace_branding(text: &str) -> String {
    PRODUCT_CONFIG
        .read()
        .map(|c| c.replace_branding(text))
        .unwrap_or_else(|_| text.to_string())
}
/// Helper function to get product config for serialization
pub fn get_product_config_json() -> serde_json::Value {
    // Get compiled features from our new module
    let compiled = crate::features::COMPILED_FEATURES;

    // Get current config
    let config = PRODUCT_CONFIG.read().ok();

    // Effective apps come straight from the .product file. There is no runtime
    // guard: the `apps` list controls what the launcher and sidebar show, and
    // nothing more. Removing an app from it does not disable its routes.
    let effective_apps: Vec<String> = config
        .as_ref()
        .map(|c| c.get_enabled_apps())
        .unwrap_or_default()
        .into_iter()
        .collect();

    match config {
        Some(c) => serde_json::json!({
            "name": c.name,
            "apps": effective_apps,
            "preview_apps": c.get_preview_apps(),
            "compiled_features": compiled,
            "version": env!("CARGO_PKG_VERSION"),
            "theme": c.theme,
            "logo": c.logo,
            "favicon": c.favicon,
            "primary_color": c.primary_color,
            "docs_url": c.docs_url,
            "copyright": c.get_copyright(),
            "search_enabled": c.search_enabled.unwrap_or(false),
            "menu_launcher_enabled": c.menu_launcher_enabled.unwrap_or(false),
            "sidebar": c.sidebar,
        }),
        None => serde_json::json!({
            "name": "General Bots",
            "apps": compiled, // If no config, show all compiled
            "preview_apps": Vec::<String>::new(),
            "compiled_features": compiled,
            "version": env!("CARGO_PKG_VERSION"),
            "theme": "sentient",
            "search_enabled": false,
            "menu_launcher_enabled": false,
            "sidebar": null,
        }),
    }
}
/// Get workspace manifest with detailed feature information
pub fn get_workspace_manifest() -> serde_json::Value {
    let manifest = crate::manifest::WorkspaceManifest::new();
    serde_json::to_value(manifest).unwrap_or_else(|_| serde_json::json!({}))
}
