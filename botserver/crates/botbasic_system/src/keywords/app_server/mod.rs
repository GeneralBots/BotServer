pub(crate) use botcore::shared::utils::get_content_type;
pub(crate) use botcore::shared::state::AppState;
pub(crate) use botcore::shared::utils::get_stack_path;
pub(crate) use axum::{ body::Body, extract::{Path, State}, http::{header, StatusCode}, response::{IntoResponse, Response}, routing::get, Router, };
pub(crate) use log::{trace, warn};
#[cfg(not(feature = "vibe"))]
pub(crate) use log::info;

/// Rewrite CDN URLs to local paths for HTMX and other vendor libraries
/// This ensures old apps with CDN references still work with local files
#[derive(Debug, serde::Deserialize)]
pub struct VendorFilePath {
    pub file_path: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct SuiteJsFilePath {
    pub file_path: String,
}

#[cfg(not(feature = "vibe"))]
#[derive(Debug, serde::Deserialize)]
pub struct AppPath {
    pub app_name: String,
}

#[cfg(not(feature = "vibe"))]
#[derive(Debug, serde::Deserialize)]
pub struct AppFilePath {
    pub app_name: String,
    pub file_path: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use botsecurity::sanitize_path_component;

    #[test]
    fn test_sanitize_path_component() {
        assert_eq!(sanitize_path_component("clinic"), "clinic");
        assert_eq!(sanitize_path_component("../etc/passwd"), "etc/passwd");
        assert_eq!(sanitize_path_component("app/../secret"), "app/secret");
        assert_eq!(sanitize_path_component("/leading/slash"), "leading/slash");
        assert_eq!(sanitize_path_component("file.html"), "file.html");
        assert_eq!(sanitize_path_component("my-app_v2"), "my-app_v2");
    }

    #[test]
    fn test_get_content_type() {
        assert_eq!(get_content_type("index.html"), "text/html; charset=utf-8");
        assert_eq!(get_content_type("styles.css"), "text/css; charset=utf-8");
        assert_eq!(
            get_content_type("app.js"),
            "application/javascript; charset=utf-8"
        );
        assert_eq!(get_content_type("image.png"), "image/png");
        assert_eq!(get_content_type("unknown.xyz"), "application/octet-stream");
    }

    #[cfg(not(feature = "vibe"))]
    #[test]
    fn test_sanitize_app_name() {
        assert_eq!(sanitize_app_name("my-app"), "my-app");
        assert_eq!(sanitize_app_name("my_app_123"), "my_app_123");
        assert_eq!(sanitize_app_name("../hack"), "hack");
        assert_eq!(sanitize_app_name("app<script>"), "appscript");
    }

    #[test]
    fn test_sanitize_file_path() {
        assert_eq!(sanitize_file_path("styles.css"), "styles.css");
        assert_eq!(sanitize_file_path("css/styles.css"), "css/styles.css");
        assert_eq!(
            sanitize_file_path("assets/img/logo.png"),
            "assets/img/logo.png"
        );
        assert_eq!(sanitize_file_path("../../../etc/passwd"), "etc/passwd");
        assert_eq!(sanitize_file_path("./styles.css"), "styles.css");
        assert_eq!(
            sanitize_file_path("path//double//slash.js"),
            "path/double/slash.js"
        );
    }
}

use std::sync::Arc;

// Stub register function — app_server provides Axum route handlers, not Rhai keywords.
// The routes are configured via configure_app_server_routes() at the botserver layer.

mod part1;
mod part2;
mod part3;

pub use part1::*;
pub use part2::*;
pub use part3::*;
