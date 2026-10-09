//! Product Configuration Module
//!
//! This module handles white-label settings loaded from the `.product` file.
//! It provides a global configuration that can be used throughout the application
//! to customize branding, enabled apps, and default theme.

pub(crate) use once_cell::sync::Lazy;
pub(crate) use std::collections::HashSet;
pub(crate) use std::fs;
pub(crate) use std::path::Path;
pub(crate) use std::sync::RwLock;
pub(crate) use tracing::{info, warn};

/// Global product configuration instance
pub static PRODUCT_CONFIG: Lazy<RwLock<ProductConfig>> =
    Lazy::new(|| RwLock::new(ProductConfig::load().unwrap_or_default()));

/// Product configuration structure
#[derive(Debug, Clone)]
pub struct ProductConfig {
    /// Product name (replaces "General Bots" throughout the application)
    pub name: String,

    /// Set of active apps
    pub apps: HashSet<String>,

    /// Applications that are only surfaced while Preview mode is on (#1348).
    /// A preview app is deliberately absent from `apps`, so it stays out of the
    /// launcher and the sidebar until the user turns the switch on.
    pub preview_apps: HashSet<String>,

    /// Default theme
    pub theme: String,

    /// Logo URL (optional)
    pub logo: Option<String>,

    /// Favicon URL (optional)
    pub favicon: Option<String>,

    /// Primary color override (optional)
    pub primary_color: Option<String>,

    /// Support email (optional)
    pub support_email: Option<String>,

    /// Documentation URL (optional)
    pub docs_url: Option<String>,

    /// Copyright text (optional)
    pub copyright: Option<String>,

    /// Search mechanism enabled (optional)
    /// Controls whether the omnibox/search toolbar is displayed in the suite
    pub search_enabled: Option<bool>,

    /// Menu launcher enabled (optional)
    /// Controls whether the apps menu launcher is displayed in the suite
    pub menu_launcher_enabled: Option<bool>,

    /// Sidebar visibility (optional)
    /// Controls whether the left sidebar is shown in the suite.
    /// When false, the sidebar is hidden and the main content takes full width.
    pub sidebar: Option<bool>,
}

/// Error type for product configuration
#[derive(Debug)]
pub enum ProductConfigError {
    IoError(std::io::Error),
    ParseError(String),
    LockError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ProductConfig::default();
        assert_eq!(config.name, "General Bots");
        assert_eq!(config.theme, "sentient");
        assert!(config.is_app_enabled("chat"));
        assert!(config.is_app_enabled("drive"));
    }

    #[test]
    fn test_parse_config() {
        let content = r#"
# Test config
name=My Custom Bot
apps=chat,drive,tasks
theme=dark
        "#;

        let config = ProductConfig::parse(content).unwrap();
        assert_eq!(config.name, "My Custom Bot");
        assert_eq!(config.theme, "dark");
        assert!(config.is_app_enabled("chat"));
        assert!(config.is_app_enabled("drive"));
        assert!(config.is_app_enabled("tasks"));
        assert!(!config.is_app_enabled("mail"));
        assert!(!config.is_app_enabled("calendar"));
    }

    #[test]
    fn test_replace_branding() {
        let config = ProductConfig {
            name: "Acme Bot".to_string(),
            ..Default::default()
        };

        assert_eq!(
            config.replace_branding("Welcome to General Bots"),
            "Welcome to Acme Bot"
        );
    }

    #[test]
    fn test_preview_apps_are_parsed_and_stay_disabled() {
        let content = "apps=chat,drive\npreview_apps=designer, marketPlace ,fraud";
        let config = ProductConfig::parse(content).unwrap();

        assert!(config.is_app_preview("designer"));
        assert!(config.is_app_preview("DESIGNER"));
        assert!(config.is_app_preview("marketplace"));
        assert!(!config.is_app_preview("chat"));

        // A preview application is not an active application: the Preview
        // switch decides what is visible, not whether the app is installed.
        assert!(!config.is_app_enabled("designer"));
        assert_eq!(
            config.get_preview_apps(),
            vec!["designer", "fraud", "marketplace"]
        );
    }

    #[test]
    fn test_missing_preview_key_leaves_the_set_empty() {
        let config = ProductConfig::parse("apps=chat").unwrap();
        assert!(config.get_preview_apps().is_empty());
        assert!(!config.is_app_preview("chat"));
    }

    #[test]
    fn test_case_insensitive_apps() {
        let content = "apps=Chat,DRIVE,Tasks";
        let config = ProductConfig::parse(content).unwrap();

        assert!(config.is_app_enabled("chat"));
        assert!(config.is_app_enabled("CHAT"));
        assert!(config.is_app_enabled("Chat"));
        assert!(config.is_app_enabled("drive"));
        assert!(config.is_app_enabled("tasks"));
    }
}

mod part1;
mod part2;

pub use part2::*;
